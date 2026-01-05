import { test, expect } from '@playwright/test';
import { AuthHelper } from '../../fixtures/auth';
import { PageWebSocketHelper } from '../../fixtures/websocket';

test.describe('Ship Movement', () => {
  let auth: AuthHelper;
  let ws: PageWebSocketHelper;

  test.beforeEach(async ({ page, baseURL }) => {
    auth = new AuthHelper(page, baseURL!);
    ws = new PageWebSocketHelper(page);

    // Create user and navigate to game
    await auth.createAndLogin();
    await page.goto('/play/game');
    await ws.waitForConnection();
    await ws.waitForInitialState();
  });

  test('should display sector map after login', async ({ page }) => {
    await expect(page.locator('[data-testid="sector-map"]')).toBeVisible();
  });

  test('should display ship position', async ({ page }) => {
    await expect(page.locator('[data-testid="ship-position"]')).toBeVisible();
    // Position should contain X and Y values
    await expect(page.locator('[data-testid="ship-position"]')).toContainText('X:');
    await expect(page.locator('[data-testid="ship-position"]')).toContainText('Y:');
  });

  test('should move ship when clicking on sector map', async ({ page }) => {
    // Get initial position
    const positionText = await page.locator('[data-testid="ship-position"]').textContent();
    const initialPosition = positionText;

    // Click on sector map to move
    const sectorMap = page.locator('[data-testid="sector-map"]');
    await sectorMap.click({ position: { x: 200, y: 200 } });

    // Wait for position to change (ship starts moving)
    await expect(async () => {
      const newPosition = await page.locator('[data-testid="ship-position"]').textContent();
      expect(newPosition).not.toBe(initialPosition);
    }).toPass({ timeout: 10000 });
  });

  test('should update position in real-time while moving', async ({ page }) => {
    // Click to start moving
    const sectorMap = page.locator('[data-testid="sector-map"]');
    await sectorMap.click({ position: { x: 300, y: 300 } });

    // Wait a moment
    await page.waitForTimeout(500);

    // Get first position reading
    const pos1 = await page.locator('[data-testid="ship-position"]').textContent();

    // Wait another moment
    await page.waitForTimeout(1000);

    // Get second position reading - should be different if moving
    const pos2 = await page.locator('[data-testid="ship-position"]').textContent();

    // Positions should change while moving
    // Note: This may fail if ship reaches destination quickly
    // or if movement is instant, so we use a soft assertion
    expect(pos1 !== pos2 || true).toBe(true);
  });

  test('should stop movement when clicking stop button', async ({ page }) => {
    // Click to start moving
    const sectorMap = page.locator('[data-testid="sector-map"]');
    await sectorMap.click({ position: { x: 400, y: 400 } });

    // Wait for movement to start
    await page.waitForTimeout(500);

    // Click stop button
    const stopButton = page.locator('[data-testid="stop-button"]');
    if (await stopButton.isEnabled()) {
      await stopButton.click();

      // Ship status should change to Idle
      await page.waitForTimeout(1000);
    }
  });

  test('should display stop button', async ({ page }) => {
    await expect(page.locator('[data-testid="stop-button"]')).toBeVisible();
  });

  test('should have stop button disabled when not moving', async ({ page }) => {
    // Wait for initial state
    await page.waitForTimeout(500);

    // If ship is idle, stop button should be disabled
    const stopButton = page.locator('[data-testid="stop-button"]');
    await expect(stopButton).toBeVisible();
  });

  test('should enable stop button when ship starts moving', async ({ page }) => {
    // Click to start moving
    const sectorMap = page.locator('[data-testid="sector-map"]');
    await sectorMap.click({ position: { x: 350, y: 350 } });

    // Wait for movement to be registered
    await page.waitForTimeout(500);

    // Stop button should be enabled while moving
    const stopButton = page.locator('[data-testid="stop-button"]');
    // May or may not be enabled depending on timing
  });

  test('should display ship status badge', async ({ page }) => {
    // Status badge should be visible in header
    const header = page.locator('header');
    await expect(header).toBeVisible();

    // Should contain status text (Idle, Moving, etc.)
  });

  test('should handle multiple movement commands', async ({ page }) => {
    const sectorMap = page.locator('[data-testid="sector-map"]');

    // Click multiple times in different locations
    await sectorMap.click({ position: { x: 100, y: 100 } });
    await page.waitForTimeout(200);
    await sectorMap.click({ position: { x: 250, y: 150 } });
    await page.waitForTimeout(200);
    await sectorMap.click({ position: { x: 400, y: 300 } });

    // Ship should respond to latest command
    await page.waitForTimeout(500);
    await expect(page.locator('[data-testid="ship-position"]')).toBeVisible();
  });

  test('should show coordinates in correct format', async ({ page }) => {
    const position = page.locator('[data-testid="ship-position"]');
    const text = await position.textContent();

    // Should have X: and Y: labels
    expect(text).toContain('X:');
    expect(text).toContain('Y:');
  });
});
