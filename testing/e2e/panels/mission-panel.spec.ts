import { test, expect } from '@playwright/test';
import { AuthHelper } from '../../fixtures/auth';
import { PageWebSocketHelper } from '../../fixtures/websocket';

test.describe('Mission Panel', () => {
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

  test('should display mission panel', async ({ page }) => {
    // Mission panel may be in a tab or sidebar
    // First check if we need to click a tab
    const missionsTab = page.locator('button:has-text("Missions")');
    if (await missionsTab.isVisible()) {
      await missionsTab.click();
    }

    await expect(page.locator('[data-testid="mission-panel"]')).toBeVisible();
  });

  test('should show available missions heading', async ({ page }) => {
    // Click missions tab if needed
    const missionsTab = page.locator('button:has-text("Missions")');
    if (await missionsTab.isVisible()) {
      await missionsTab.click();
    }

    const missionPanel = page.locator('[data-testid="mission-panel"]');
    await expect(missionPanel).toContainText('Available Missions');
  });

  test('should show active mission section', async ({ page }) => {
    // Click missions tab if needed
    const missionsTab = page.locator('button:has-text("Missions")');
    if (await missionsTab.isVisible()) {
      await missionsTab.click();
    }

    const missionPanel = page.locator('[data-testid="mission-panel"]');
    await expect(missionPanel).toContainText('Active Mission');
  });

  test('should display empty state when no missions available', async ({ page }) => {
    // Click missions tab if needed
    const missionsTab = page.locator('button:has-text("Missions")');
    if (await missionsTab.isVisible()) {
      await missionsTab.click();
    }

    const missionPanel = page.locator('[data-testid="mission-panel"]');

    // Either missions are available or empty state is shown
    const hasMissions = await missionPanel.locator('button:has-text("Accept")').isVisible().catch(() => false);
    const hasEmptyState = await missionPanel.locator('text=/No missions available|Check back later/').isVisible().catch(() => false);

    expect(hasMissions || hasEmptyState).toBe(true);
  });
});
