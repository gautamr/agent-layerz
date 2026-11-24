//! A shortest edit script between two sequences, by Myers' linear-space algorithm (spec §10.3
//! "alz diff"): the same two sequences always give the same script.

/// What one step of the script does to the old sequence to reach the new one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Edit {
    /// The element is in both sequences.
    Keep,
    /// The element is only in the old sequence.
    Remove,
    /// The element is only in the new sequence.
    Add,
}

/// A shortest script from `old` to `new`; within a run of changes between two kept elements the
/// removals come first, then the additions.
///
/// The search costs about `(old + new) * d` steps for a shortest script of `d` edits, counted
/// after the shared first and last elements are set aside. When that would exceed `cost_cap`, the
/// elements between the shared ends are removed as a whole and the new ones added as a whole: a
/// valid script that is not a shortest one.
pub fn script<T: Eq>(old: &[T], new: &[T], cost_cap: usize) -> Vec<Edit> {
    let (old_middle, new_middle) = without_shared_ends(old, new);
    let size = (old_middle.len() + new_middle.len()).max(1);
    let limit = (cost_cap / size).max(1).cast_signed();
    removals_first(&search(old, new, limit))
}

/// Reorders each run of changes: every removal, then every addition.
fn removals_first(edits: &[Edit]) -> Vec<Edit> {
    edits
        .chunk_by(|left, right| (*left == Edit::Keep) == (*right == Edit::Keep))
        .flat_map(|run| {
            let removed = run.iter().filter(|edit| **edit == Edit::Remove).count();
            let added = run.iter().filter(|edit| **edit == Edit::Add).count();
            let kept = run.iter().filter(|edit| **edit == Edit::Keep).count();
            std::iter::repeat_n(Edit::Keep, kept)
                .chain(std::iter::repeat_n(Edit::Remove, removed))
                .chain(std::iter::repeat_n(Edit::Add, added))
        })
        .collect()
}

fn common_prefix<T: Eq>(old: &[T], new: &[T]) -> usize {
    old.iter()
        .zip(new)
        .take_while(|(left, right)| left == right)
        .count()
}

fn common_suffix<T: Eq>(old: &[T], new: &[T]) -> usize {
    old.iter()
        .rev()
        .zip(new.iter().rev())
        .take_while(|(left, right)| left == right)
        .count()
}

fn tail<T>(items: &[T], from: usize) -> &[T] {
    items.get(from..).unwrap_or_default()
}

fn head<T>(items: &[T], to: usize) -> &[T] {
    items.get(..to).unwrap_or_default()
}

/// What is left of both sequences once the elements they share at the start and at the end are
/// set aside.
fn without_shared_ends<'a, T: Eq>(old: &'a [T], new: &'a [T]) -> (&'a [T], &'a [T]) {
    let front = common_prefix(old, new);
    let (old, new) = (tail(old, front), tail(new, front));
    let back = common_suffix(old, new);
    (
        head(old, old.len().saturating_sub(back)),
        head(new, new.len().saturating_sub(back)),
    )
}

/// Divide and conquer: strip what both ends share, split at the middle snake of a shortest path
/// of at most `limit` edits, and solve both halves. Past the limit the middle is replaced whole.
fn search<T: Eq>(old: &[T], new: &[T], limit: isize) -> Vec<Edit> {
    let front = common_prefix(old, new);
    let (old_tail, new_tail) = (tail(old, front), tail(new, front));
    let back = common_suffix(old_tail, new_tail);
    let (old, new) = without_shared_ends(old, new);
    let middle = if old.is_empty() {
        vec![Edit::Add; new.len()]
    } else if new.is_empty() {
        vec![Edit::Remove; old.len()]
    } else {
        match middle_snake(old, new, limit) {
            Some((x, y)) => search(head(old, x), head(new, y), limit)
                .into_iter()
                .chain(search(tail(old, x), tail(new, y), limit))
                .collect(),
            None => std::iter::repeat_n(Edit::Remove, old.len())
                .chain(std::iter::repeat_n(Edit::Add, new.len()))
                .collect(),
        }
    };
    std::iter::repeat_n(Edit::Keep, front)
        .chain(middle)
        .chain(std::iter::repeat_n(Edit::Keep, back))
        .collect()
}

/// The furthest x reached on each diagonal `k = x - y`, for one direction of the search.
struct Reach {
    values: Vec<isize>,
    offset: isize,
}

impl Reach {
    fn new(bound: isize) -> Self {
        Self {
            values: vec![0; (2 * bound + 3).cast_unsigned()],
            offset: bound + 1,
        }
    }

    fn at(&self, diagonal: isize) -> isize {
        usize::try_from(diagonal + self.offset)
            .ok()
            .and_then(|index| self.values.get(index))
            .copied()
            .unwrap_or_default()
    }

    fn reach(&mut self, diagonal: isize, x: isize) {
        if let Some(slot) = usize::try_from(diagonal + self.offset)
            .ok()
            .and_then(|index| self.values.get_mut(index))
        {
            *slot = x;
        }
    }

    /// Where a path of `d` edits arrives on `diagonal`: from the diagonal above when that went
    /// further, else from the one below.
    fn arrival(&self, diagonal: isize, d: isize) -> isize {
        if diagonal == -d || (diagonal != d && self.at(diagonal - 1) < self.at(diagonal + 1)) {
            self.at(diagonal + 1)
        } else {
            self.at(diagonal - 1) + 1
        }
    }
}

/// A point `(x, y)` on a shortest path from the start to the end of two non-empty sequences that
/// is neither the start nor the end unless the sequences differ by one element at most; `None`
/// when no path of at most `limit` edits exists.
fn middle_snake<T: Eq>(old: &[T], new: &[T], limit: isize) -> Option<(usize, usize)> {
    let n = old.len().cast_signed();
    let m = new.len().cast_signed();
    let delta = n - m;
    let odd = delta & 1 == 1;
    let bound = ((n + m + 1) / 2 + 1).min(limit);
    let (mut forward, mut backward) = (Reach::new(bound), Reach::new(bound));
    forward.reach(1, 0);
    backward.reach(1, 0);
    for d in 0..=bound {
        for k in (-d..=d).rev().step_by(2) {
            let x0 = forward.arrival(k, d);
            let y0 = x0 - k;
            let slid = slide(tail(old, x0.cast_unsigned()), tail(new, y0.cast_unsigned()));
            forward.reach(k, x0 + slid);
            if odd && (k - delta).abs() < d && forward.at(k) + backward.at(delta - k) >= n {
                return Some((x0.cast_unsigned(), y0.cast_unsigned()));
            }
        }
        for k in (-d..=d).rev().step_by(2) {
            let x0 = backward.arrival(k, d);
            let y0 = x0 - k;
            let slid = slide_back(
                head(old, (n - x0).cast_unsigned()),
                head(new, (m - y0).cast_unsigned()),
            );
            backward.reach(k, x0 + slid);
            if !odd && (k - delta).abs() <= d && backward.at(k) + forward.at(delta - k) >= n {
                return Some((
                    (n - x0 - slid).cast_unsigned(),
                    (m - y0 - slid).cast_unsigned(),
                ));
            }
        }
    }
    None
}

fn slide<T: Eq>(old: &[T], new: &[T]) -> isize {
    common_prefix(old, new).cast_signed()
}

fn slide_back<T: Eq>(old: &[T], new: &[T]) -> isize {
    common_suffix(old, new).cast_signed()
}
