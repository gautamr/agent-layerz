import { withRunner } from "../account-workspace";
import { layersInProcess } from "./in-process";

/**
 * `alz` and `built` of the account workspace, run in this process on the TypeScript
 * implementation, for the reference cases that trace or fake what the code does. The acceptance
 * suite uses the spawning versions in test/account-workspace.ts. Deleted with src/ at the cutover.
 */
export const { alz, built } = withRunner((ws, dir, args, env) =>
  layersInProcess(ws, dir, args, env),
);
