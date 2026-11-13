import { expect, test, type Page } from "@playwright/test";
import { PNG } from "pngjs";

/** The page's URL state, mirrored by app.js for the tests. */
interface DemoState {
  frames: number;
  backend: string;
  adapter: string;
  attractorId: string;
  attractorName: string;
  seed: string;
}

function demoState(page: Page) {
  return page.evaluate(() => (window as unknown as { __attractorDemo: DemoState }).__attractorDemo);
}

async function waitForFrames(page: Page, minimum = 10) {
  await page.waitForFunction(
    (count) =>
      (window as unknown as { __attractorDemo?: DemoState }).__attractorDemo?.frames >= count,
    minimum,
    { timeout: 30_000 },
  );
}

/** Count pixels that are not the black background. */
function litPixels(screenshot: Buffer): number {
  const image = PNG.sync.read(screenshot);
  let lit = 0;
  for (let index = 0; index < image.data.length; index += 4) {
    const brightness = image.data[index] + image.data[index + 1] + image.data[index + 2];
    if (brightness > 24) {
      lit += 1;
    }
  }
  return lit;
}

/**
 * Hide the control panel and the DOM overlay.
 *
 * An element screenshot is a clip of the composited page, so anything painted
 * over the canvas would be counted as "lit". Hiding the chrome makes the
 * pixel assertions measure the GPU output alone.
 */
async function hideChrome(page: Page) {
  await page.evaluate(() => {
    for (const id of ["controls", "overlay"]) {
      document.getElementById(id)?.setAttribute("style", "visibility: hidden");
    }
  });
}

/** Record console errors and uncaught exceptions for the "clean console" gate. */
function collectErrors(page: Page): string[] {
  const errors: string[] = [];
  page.on("console", (message) => {
    if (message.type() === "error") {
      errors.push(message.text());
    }
  });
  page.on("pageerror", (error) => errors.push(String(error)));
  return errors;
}

test("the forced WebGL2 fallback renders an animated, non-blank canvas", async ({ page }) => {
  const errors = collectErrors(page);
  await page.goto("/index.html?backend=webgl2&attractor=lorenz&seed=42");
  await waitForFrames(page);

  const state = await demoState(page);
  expect(state.backend).toBe("webgl2");
  expect(state.attractorId).toBe("lorenz");
  expect(state.seed).toBe("42");
  await expect(page.locator("#overlay")).toContainText("Lorenz");

  await hideChrome(page);
  const first = await page.locator("#stage").screenshot();
  expect(litPixels(first)).toBeGreaterThan(100);

  await page.waitForTimeout(700);
  const second = await page.locator("#stage").screenshot();
  expect(litPixels(second)).toBeGreaterThan(100);
  expect(Buffer.compare(first, second)).not.toBe(0);

  expect(errors).toEqual([]);
});

test("the automatic backend renders through WebGPU or WebGL2", async ({ page }) => {
  const errors = collectErrors(page);
  await page.goto("/index.html?seed=7");
  await waitForFrames(page);

  const state = await demoState(page);
  expect(["webgpu", "webgl2"]).toContain(state.backend);
  expect(state.attractorId.length).toBeGreaterThan(0);

  await hideChrome(page);
  const screenshot = await page.locator("#stage").screenshot();
  expect(litPixels(screenshot)).toBeGreaterThan(100);
  expect(errors).toEqual([]);
});
