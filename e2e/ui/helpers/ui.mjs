// Small helpers over WebDriver. The app is driven by what a player sees (the English labels, the
// roles the app already declares) and by the ids and data-testid attributes it already has. A
// step that cannot be driven that way is a request for a new test id, made to the frontend owner
// first, not a CSS selector against the styling.
const SECOND = 1000;

const xpathLiteral = (text) => (text.includes('"') ? `'${text}'` : `"${text}"`);

export function buttonNamed(name) {
  return $(`//button[normalize-space(.)=${xpathLiteral(name)}]`);
}

export function buttonStartingWith(prefix) {
  return $(`//button[starts-with(normalize-space(.), ${xpathLiteral(prefix)})]`);
}

export async function clickWhenReady(element, timeout = 30 * SECOND) {
  await element.waitForClickable({ timeout });
  await element.click();
}

export async function waitForRoute(fragment, timeout = 120 * SECOND) {
  await browser.waitUntil(async () => (await browser.getUrl()).includes(fragment), {
    timeout,
    timeoutMsg: `the app never reached ${fragment}`,
  });
}

export async function typeInto(element, text) {
  await element.waitForDisplayed({ timeout: 30 * SECOND });
  await element.setValue(text);
}
