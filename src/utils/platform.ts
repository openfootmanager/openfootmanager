/**
 * Returns true when the app is running on an Android device (Tauri Android
 * WebView or a mobile browser). Desktop-only window APIs (close interception,
 * window destroy) must be skipped there.
 */
export function isAndroid(): boolean {
  return /android/i.test(navigator.userAgent);
}
