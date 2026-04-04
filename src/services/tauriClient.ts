import { invoke } from "@tauri-apps/api/core";

type CommandArgs = object;

export async function invokeCommand<TResult>(
    command: string,
    args?: CommandArgs,
): Promise<TResult> {
    if (args) {
        return invoke<TResult>(command, args);
    }

    return invoke<TResult>(command);
}

export async function invokeVoidCommand(
    command: string,
    args?: CommandArgs,
): Promise<void> {
    await invokeCommand<void>(command, args);
}