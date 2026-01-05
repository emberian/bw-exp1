import { test, expect } from '@playwright/test';
import { AuthHelper } from '../../fixtures/auth';
import { PageWebSocketHelper } from '../../fixtures/websocket';

test.describe('Combat System', () => {
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

  test('should display combat log panel', async ({ page }) => {
    // Click Combat tab to show combat log
    const combatTab = page.locator('button:has-text("Combat")');
    if (await combatTab.isVisible()) {
      await combatTab.click();
    }

    await expect(page.locator('[data-testid="combat-log"]')).toBeVisible();
  });

  test('should show "No active combat" when not in combat', async ({ page }) => {
    // Click Combat tab
    const combatTab = page.locator('button:has-text("Combat")');
    if (await combatTab.isVisible()) {
      await combatTab.click();
    }

    const combatLog = page.locator('[data-testid="combat-log"]');
    await expect(combatLog).toContainText('No active combat');
  });

  test('should show alert button in footer', async ({ page }) => {
    await expect(page.locator('[data-testid="alert-button"]')).toBeVisible();
  });

  test('should have alert button disabled when not in combat', async ({ page }) => {
    const alertButton = page.locator('[data-testid="alert-button"]');
    await expect(alertButton).toBeDisabled();
  });

  test('should show combat header in combat log', async ({ page }) => {
    // Click Combat tab
    const combatTab = page.locator('button:has-text("Combat")');
    if (await combatTab.isVisible()) {
      await combatTab.click();
    }

    const combatLog = page.locator('[data-testid="combat-log"]');
    await expect(combatLog).toContainText('Combat');
  });
});
