import { invoke } from "@tauri-apps/api/core";
import type { InvokeArgs } from "@tauri-apps/api/core";

export async function invokeCommand<TResult>(
    command: string,
    args?: object,
): Promise<TResult> {
    if (args) {
        return invoke<TResult>(command, args as InvokeArgs);
    }

    return invoke<TResult>(command);
}

export async function invokeVoidCommand(
    command: string,
    args?: object,
): Promise<void> {
    if (args) {
        await invokeCommand<void>(command, args);
        return;
    }

    await invokeCommand<void>(command);
}