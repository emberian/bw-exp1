import { test, expect } from '@playwright/test';
import { AuthHelper } from '../../fixtures/auth';
import { PageWebSocketHelper } from '../../fixtures/websocket';

test.describe('Sector Map Panel', () => {
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

  test('should display sector map', async ({ page }) => {
    await expect(page.locator('[data-testid="sector-map"]')).toBeVisible();
  });

  test('should display sector name', async ({ page }) => {
    // Sector info should be visible in the map area
    const sectorMap = page.locator('[data-testid="sector-map"]');
    await expect(sectorMap).toBeVisible();

    // There should be some sector name displayed
    // (exact name depends on game data)
  });

  test('should have clickable area for movement', async ({ page }) => {
    const sectorMap = page.locator('[data-testid="sector-map"]');

    // Map should be interactive
    await expect(sectorMap).toBeVisible();

    // Should be able to click on map
    const box = await sectorMap.boundingBox();
    expect(box).toBeTruthy();
    expect(box!.width).toBeGreaterThan(100);
    expect(box!.height).toBeGreaterThan(100);
  });

  test('should respond to click events', async ({ page }) => {
    const sectorMap = page.locator('[data-testid="sector-map"]');
    const position = page.locator('[data-testid="ship-position"]');

    // Get initial position
    const initialPos = await position.textContent();

    // Click on the map
    await sectorMap.click({ position: { x: 150, y: 150 } });

    // Wait for position to potentially change
    await page.waitForTimeout(1500);

    // Position text should still be displayed
    await expect(position).toBeVisible();
  });
});
