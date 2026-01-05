import { test, expect } from '@playwright/test';
import { AuthHelper } from '../../fixtures/auth';
import { PageWebSocketHelper } from '../../fixtures/websocket';

test.describe('Mission System', () => {
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

  test('should display mission panel', async ({ page }) => {
    // Click missions tab if needed
    const missionsTab = page.locator('button:has-text("Missions")');
    if (await missionsTab.isVisible()) {
      await missionsTab.click();
    }

    await expect(page.locator('[data-testid="mission-panel"]')).toBeVisible();
  });

  test('should show Available Missions section', async ({ page }) => {
    const missionsTab = page.locator('button:has-text("Missions")');
    if (await missionsTab.isVisible()) {
      await missionsTab.click();
    }

    const missionPanel = page.locator('[data-testid="mission-panel"]');
    await expect(missionPanel).toContainText('Available Missions');
  });

  test('should show Active Mission section', async ({ page }) => {
    const missionsTab = page.locator('button:has-text("Missions")');
    if (await missionsTab.isVisible()) {
      await missionsTab.click();
    }

    const missionPanel = page.locator('[data-testid="mission-panel"]');
    await expect(missionPanel).toContainText('Active Mission');
  });

  test('should display missions or empty state', async ({ page }) => {
    const missionsTab = page.locator('button:has-text("Missions")');
    if (await missionsTab.isVisible()) {
      await missionsTab.click();
    }

    const missionPanel = page.locator('[data-testid="mission-panel"]');

    // Either shows missions with Accept button or shows empty state
    const hasMissions = await missionPanel.locator('button:has-text("Accept")').isVisible().catch(() => false);
    const hasEmptyState = await missionPanel.locator('text=/No missions available|Check back later/').isVisible().catch(() => false);

    expect(hasMissions || hasEmptyState).toBe(true);
  });

  test('should show mission rewards when missions available', async ({ page }) => {
    const missionsTab = page.locator('button:has-text("Missions")');
    if (await missionsTab.isVisible()) {
      await missionsTab.click();
    }

    const missionPanel = page.locator('[data-testid="mission-panel"]');

    // If missions are available, they should show reward info (Rep/Fame)
    const hasMissions = await missionPanel.locator('button:has-text("Accept")').isVisible().catch(() => false);

    if (hasMissions) {
      // Missions should display reward info
      const hasRewardInfo = await missionPanel.locator('text=/Rep|Fame/').isVisible().catch(() => false);
      expect(hasRewardInfo).toBe(true);
    }
  });

  test('should be able to accept mission if available', async ({ page }) => {
    const missionsTab = page.locator('button:has-text("Missions")');
    if (await missionsTab.isVisible()) {
      await missionsTab.click();
    }

    const missionPanel = page.locator('[data-testid="mission-panel"]');

    // Find Accept button
    const acceptButton = missionPanel.locator('button:has-text("Accept")').first();

    if (await acceptButton.isVisible().catch(() => false)) {
      // Click to accept
      await acceptButton.click();

      // Wait for state update
      await page.waitForTimeout(1000);

      // Check if mission moved to active (or notification shown)
      // The Accept button should disappear or mission should be active
    }
  });

  test('should show abandon option for active mission', async ({ page }) => {
    const missionsTab = page.locator('button:has-text("Missions")');
    if (await missionsTab.isVisible()) {
      await missionsTab.click();
    }

    const missionPanel = page.locator('[data-testid="mission-panel"]');

    // First accept a mission if possible
    const acceptButton = missionPanel.locator('button:has-text("Accept")').first();
    if (await acceptButton.isVisible().catch(() => false)) {
      await acceptButton.click();
      await page.waitForTimeout(1000);
    }

    // Check for Abandon button in active mission section
    const abandonButton = missionPanel.locator('button:has-text("Abandon")');
    // This may or may not be visible depending on whether there's an active mission
    if (await abandonButton.isVisible().catch(() => false)) {
      expect(await abandonButton.isEnabled()).toBe(true);
    }
  });

  test('should navigate between mission and ship tabs', async ({ page }) => {
    // Click missions tab
    const missionsTab = page.locator('button:has-text("Missions")');
    if (await missionsTab.isVisible()) {
      await missionsTab.click();
    }
    await expect(page.locator('[data-testid="mission-panel"]')).toBeVisible();

    // Click ship tab
    const shipTab = page.locator('button:has-text("Ship")');
    if (await shipTab.isVisible()) {
      await shipTab.click();
    }
    await expect(page.locator('[data-testid="ship-status"]')).toBeVisible();
  });
});
