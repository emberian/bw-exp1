import { test, expect } from '@playwright/test';
import { AuthHelper } from '../../fixtures/auth';
import { PageWebSocketHelper } from '../../fixtures/websocket';

test.describe('Ship Status Panel', () => {
  let auth: AuthHelper;
  let ws: PageWebSocketHelper;

  test.beforeEach(async ({ page, baseURL }) => {
    auth = new AuthHelper(page, baseURL!);
    ws = new PageWebSocketHelper(page);

    await auth.createAndLogin();
    await page.goto('/play/game');
    await ws.waitForConnection();
    await ws.waitForInitialState();
  });

  test('should display ship status panel', async ({ page }) => {
    // Ship status may be in a tab or always visible
    const shipTab = page.locator('button:has-text("Ship")');
    if (await shipTab.isVisible()) {
      await shipTab.click();
    }

    await expect(page.locator('[data-testid="ship-status"]')).toBeVisible();
  });

  test('should show Ship Status heading', async ({ page }) => {
    const shipTab = page.locator('button:has-text("Ship")');
    if (await shipTab.isVisible()) {
      await shipTab.click();
    }

    const shipStatus = page.locator('[data-testid="ship-status"]');
    await expect(shipStatus).toContainText('Ship Status');
  });

  test('should display hull status', async ({ page }) => {
    const shipTab = page.locator('button:has-text("Ship")');
    if (await shipTab.isVisible()) {
      await shipTab.click();
    }

    const shipStatus = page.locator('[data-testid="ship-status"]');
    await expect(shipStatus).toContainText('Hull');
  });

  test('should display shields status', async ({ page }) => {
    const shipTab = page.locator('button:has-text("Ship")');
    if (await shipTab.isVisible()) {
      await shipTab.click();
    }

    const shipStatus = page.locator('[data-testid="ship-status"]');
    await expect(shipStatus).toContainText('Shields');
  });

  test('should display systems section', async ({ page }) => {
    const shipTab = page.locator('button:has-text("Ship")');
    if (await shipTab.isVisible()) {
      await shipTab.click();
    }

    const shipStatus = page.locator('[data-testid="ship-status"]');
    await expect(shipStatus).toContainText('Systems');
  });

  test('should display ship position', async ({ page }) => {
    const shipTab = page.locator('button:has-text("Ship")');
    if (await shipTab.isVisible()) {
      await shipTab.click();
    }

    await expect(page.locator('[data-testid="ship-position"]')).toBeVisible();
  });

  test('should display position coordinates', async ({ page }) => {
    const shipTab = page.locator('button:has-text("Ship")');
    if (await shipTab.isVisible()) {
      await shipTab.click();
    }

    const position = page.locator('[data-testid="ship-position"]');
    await expect(position).toContainText('X:');
    await expect(position).toContainText('Y:');
  });
});
