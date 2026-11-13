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
  stage: string;
  error: string;
}

function demoState(page: Page) {
  return page.evaluate(() => (window as unknown as { __attractorDemo: DemoState }).__attractorDemo);
}

/** What the browser can actually give the demo, for a failure message. */
function probeEnvironment(page: Page) {
  return page.evaluate(() => {
    const canvas = document.createElement("canvas");
    const context = canvas.getContext("webgl2");
    return {
      webgl2: context !== null,
      webgl2Renderer: context?.getParameter(context.VERSION) ?? "",
      webgpu: "gpu" in navigator,
    };
  });
}

/**
 * Wait for the frame loop to advance.
 *
 * A bare timeout would not say whether the browser lacks WebGL2, the demo
 * failed to start or the first frame raised, so a failure reports the page
 * state, the environment and the errors collected so far.
 */
async function waitForFrames(page: Page, errors: string[], minimum = 10) {
  try {
    await page.waitForFunction(
      (count) =>
        (window as unknown as { __attractorDemo?: DemoState }).__attractorDemo?.frames >= count,
      minimum,
      { timeout: 30_000 },
    );
  } catch {
    const state = await demoState(page).catch(() => null);
    const environment = await probeEnvironment(page).catch(() => null);
    throw new Error(
      [
        `the demo did not reach ${minimum} frames`,
        `state: ${JSON.stringify(state)}`,
        `environment: ${JSON.stringify(environment)}`,
        `page errors: ${errors.length > 0 ? errors.join(" | ") : "(none)"}`,
      ].join("\n"),
    );
  }
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
  await waitForFrames(page, errors);

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
  await waitForFrames(page, errors);

  const state = await demoState(page);
  // Which backend the runner picked is the first thing to read when this job
  // goes red, and the successful run is the only place it is recorded.
  console.log(`automatic backend: ${state.backend} (${state.adapter})`);
  expect(["webgpu", "webgl2"]).toContain(state.backend);
  expect(state.attractorId.length).toBeGreaterThan(0);

  await hideChrome(page);
  const screenshot = await page.locator("#stage").screenshot();
  expect(litPixels(screenshot)).toBeGreaterThan(100);
  expect(errors).toEqual([]);
});
