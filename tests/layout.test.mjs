import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";
import { formatQuote, normalizeQuote } from "../src/quote.js";

const projectFile = (path) => new URL(`../${path}`, import.meta.url);

test("main window uses a 370x600 compact layout with a bottom quote", async () => {
  const [html, css, app, configText] = await Promise.all([
    readFile(projectFile("src/index.html"), "utf8"),
    readFile(projectFile("src/style.css"), "utf8"),
    readFile(projectFile("src/app.js"), "utf8"),
    readFile(projectFile("src-tauri/tauri.conf.json"), "utf8"),
  ]);
  const config = JSON.parse(configText);
  const mainWindow = config.app.windows.find((window) => window.label === "main");

  assert.equal((html.match(/class="[^"]*workspace-panel[^"]*"/g) ?? []).length, 2);
  assert.equal((html.match(/id="quote-card"/g) ?? []).length, 1);
  assert.equal((html.match(/id="quote-author"/g) ?? []).length, 1);
  assert.match(html, /class="[^"]*quote-banner[^"]*"/);
  assert.ok(html.indexOf("quote-banner") > html.indexOf("card-grid"));
  assert.match(html, /<dialog id="settings-dialog" aria-labelledby="settings-dialog-title">/);
  assert.match(html, /<h2 id="settings-dialog-title"/);
  assert.match(html, /id="settings-btn"/);
  assert.match(html, /id="settings-close"/);
  const timerStart = html.indexOf("timer-card");
  const tasksStart = html.indexOf("tasks-panel");
  const settingsStart = html.indexOf('id="settings-dialog"');
  const settingsEnd = html.indexOf("</dialog>", settingsStart);
  assert.ok(html.indexOf('id="focus-minutes"') > timerStart);
  assert.ok(html.indexOf('id="focus-minutes"') < tasksStart);
  assert.ok(html.indexOf('id="break-minutes"') > timerStart);
  assert.ok(!html.slice(settingsStart, settingsEnd).includes('id="focus-minutes"'));
  assert.doesNotMatch(html, /id="quote-inline"/);
  assert.match(css, /\.card-grid\s*{[^}]*grid-template-columns:\s*1fr;/s);
  assert.match(css, /#settings-dialog/);
  const aboutDialogCss = css.match(/#about-dialog\s*{([^}]*)}/s)?.[1] ?? "";
  const settingsDialogCss = css.match(/#settings-dialog\s*{([^}]*)}/s)?.[1] ?? "";
  assert.doesNotMatch(aboutDialogCss, /transform:/);
  assert.doesNotMatch(settingsDialogCss, /transform:/);
  assert.match(css, /\.quote-banner\s*{[^}]*margin-top:\s*10px;/s);
  assert.doesNotMatch(css, /\.quote-banner\s*{[^}]*\n\s*height:\s*\d+px;/s);
  assert.doesNotMatch(app, /quoteInline/);
  assert.match(app, /quoteCard\.textContent = quote/);
  assert.match(app, /quoteAuthor\.textContent = quote/);
  assert.match(app, /settingsDialog\.showModal/);
  assert.match(app, /persistTimerDurations/);
  assert.ok(
    app.indexOf('listen("pomodoro://state"') < app.indexOf("await loadSnapshot()"),
    "state listener must be registered before loading the initial snapshot",
  );
  assert.equal(mainWindow.width, 370);
  assert.equal(mainWindow.height, 600);
  assert.ok(mainWindow.minWidth <= 370);
  assert.ok(mainWindow.minHeight <= 600);
});

test("quote formatting handles missing and whitespace-only data", () => {
  assert.equal(normalizeQuote(null), null);
  assert.equal(normalizeQuote({ text: "   ", author: "Nobody" }), null);
  assert.deepEqual(normalizeQuote({ text: "  Stay focused.  ", author: "  Author  " }), {
    text: "Stay focused.",
    author: "Author",
  });
  assert.equal(formatQuote({ text: "Stay focused.", author: "" }, "Fallback"), '"Stay focused."');
  assert.equal(formatQuote({ text: null }, "Fallback"), "Fallback");
});
