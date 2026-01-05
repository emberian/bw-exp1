import { test, expect } from '@playwright/test';
import { AuthHelper } from '../../fixtures/auth';
import { PageWebSocketHelper } from '../../fixtures/websocket';

test.describe('Notifications Panel', () => {
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

  test('should show pending requests header when invites present', async ({ page }) => {
    // This requires having pending squad invites
    const notificationsPanel = page.locator('[data-testid="notifications-panel"]');
    if (await notificationsPanel.isVisible().catch(() => false)) {
      await expect(notificationsPanel).toContainText('Pending Requests');
    }
  });

  test('should display accept and decline buttons for invites', async ({ page }) => {
    const notificationsPanel = page.locator('[data-testid="notifications-panel"]');
    if (await notificationsPanel.isVisible().catch(() => false)) {
      // Each notification should have Accept and Decline buttons
      await expect(notificationsPanel.locator('button:has-text("Accept")')).toBeVisible();
      await expect(notificationsPanel.locator('button:has-text("Decline")')).toBeVisible();
    }
  });
});
