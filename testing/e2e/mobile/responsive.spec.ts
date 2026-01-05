import { test, expect, devices } from '@playwright/test';
import { AuthHelper } from '../../fixtures/auth';
import { PageWebSocketHelper } from '../../fixtures/websocket';

// Mobile viewport tests
test.describe('Mobile Responsive Layout', () => {
  // Use iPhone 13 viewport for mobile tests
  test.use({ ...devices['iPhone 13'] });

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

  test('should display mobile bottom navigation', async ({ page }) => {
    // Mobile nav should be visible on small screens
    const mobileNav = page.locator('nav').filter({ hasText: 'Map' });
    await expect(mobileNav).toBeVisible();
  });

  test('should show Map button in mobile nav', async ({ page }) => {
    const mapButton = page.locator('button:has-text("Map")');
    await expect(mapButton).toBeVisible();
  });

  test('should show Missions button in mobile nav', async ({ page }) => {
    const missionsButton = page.locator('button:has-text("Missions")');
    await expect(missionsButton).toBeVisible();
  });

  test('should show Ship button in mobile nav', async ({ page }) => {
    const shipButton = page.locator('button:has-text("Ship")');
    await expect(shipButton).toBeVisible();
  });

  test('should show Combat button in mobile nav', async ({ page }) => {
    const combatButton = page.locator('button:has-text("Combat")');
    await expect(combatButton).toBeVisible();
  });

  test('should show More button in mobile nav', async ({ page }) => {
    const moreButton = page.locator('button:has-text("More")');
    await expect(moreButton).toBeVisible();
  });

  test('should open missions panel when clicking Missions', async ({ page }) => {
    const missionsButton = page.locator('button:has-text("Missions")');
    await missionsButton.click();

    // Mission panel should become visible as full-screen overlay
    await expect(page.locator('[data-testid="mission-panel"]')).toBeVisible();
  });

  test('should open ship panel when clicking Ship', async ({ page }) => {
    const shipButton = page.locator('button:has-text("Ship")');
    await shipButton.click();

    // Ship status should become visible
    await expect(page.locator('[data-testid="ship-status"]')).toBeVisible();
  });

  test('should open combat panel when clicking Combat', async ({ page }) => {
    const combatButton = page.locator('button:has-text("Combat")');
    await combatButton.click();

    // Combat log should become visible
    await expect(page.locator('[data-testid="combat-log"]')).toBeVisible();
  });

  test('should show More menu dropdown', async ({ page }) => {
    const moreButton = page.locator('button:has-text("More")');
    await moreButton.click();

    // Dropdown should appear with Squadron and Comms options
    await expect(page.locator('text=Squadron').last()).toBeVisible();
    await expect(page.locator('text=Comms').last()).toBeVisible();
  });

  test('should open squadron panel from More menu', async ({ page }) => {
    const moreButton = page.locator('button:has-text("More")');
    await moreButton.click();

    // Click Squadron option in dropdown
    const squadronOption = page.locator('button:has-text("Squadron")').last();
    await squadronOption.click();

    // Squadron panel should be visible
    await expect(page.locator('[data-testid="squadron-panel"]')).toBeVisible();
  });

  test('should open comms panel from More menu', async ({ page }) => {
    const moreButton = page.locator('button:has-text("More")');
    await moreButton.click();

    // Click Comms option in dropdown
    const commsOption = page.locator('button:has-text("Comms")').last();
    await commsOption.click();

    // Comms panel should be visible
    await expect(page.locator('[data-testid="comms-panel"]')).toBeVisible();
  });

  test('should show close button on panel overlay', async ({ page }) => {
    const missionsButton = page.locator('button:has-text("Missions")');
    await missionsButton.click();
    await expect(page.locator('[data-testid="mission-panel"]')).toBeVisible();

    // Close button should be visible in the panel overlay
    const closeButton = page.locator('aside button').first();
    await expect(closeButton).toBeVisible();
  });

  test('should close panel when clicking close button', async ({ page }) => {
    const missionsButton = page.locator('button:has-text("Missions")');
    await missionsButton.click();
    await expect(page.locator('[data-testid="mission-panel"]')).toBeVisible();

    // Find and click close button
    const closeButton = page.locator('aside button').first();
    await closeButton.click();

    // Panel should close
    await expect(page.locator('[data-testid="mission-panel"]')).not.toBeVisible();
  });

  test('should show condensed resource bar on mobile header', async ({ page }) => {
    // Mobile header should show condensed resources
    const header = page.locator('header');
    await expect(header).toBeVisible();

    // Should show fuel percentage
    await expect(header.locator('text="%"')).toBeVisible();
  });

  test('should display sector map on mobile', async ({ page }) => {
    await expect(page.locator('[data-testid="sector-map"]')).toBeVisible();
  });

  test('should handle touch on sector map', async ({ page }) => {
    const sectorMap = page.locator('[data-testid="sector-map"]');
    await expect(sectorMap).toBeVisible();

    // Tap on map
    await sectorMap.tap({ position: { x: 100, y: 100 } });
    await page.waitForTimeout(500);

    // Position should still be displayed
    await expect(page.locator('[data-testid="ship-position"]')).toBeVisible();
  });

  test('should show mobile logout icon instead of text', async ({ page }) => {
    // On mobile, logout button shows icon only
    const logoutButton = page.locator('[data-testid="logout-button"]');
    await expect(logoutButton).toBeVisible();

    // Should have SVG icon
    await expect(logoutButton.locator('svg')).toBeVisible();
  });
});

// Desktop viewport tests
test.describe('Desktop Layout', () => {
  test.use({ viewport: { width: 1280, height: 800 } });

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

  test('should hide mobile bottom nav on desktop', async ({ page }) => {
    // Mobile nav should be hidden on larger screens
    const mobileNav = page.locator('nav.md\\:hidden');
    await expect(mobileNav).not.toBeVisible();
  });

  test('should show sidebar tabs on desktop', async ({ page }) => {
    // Left sidebar tabs
    await expect(page.locator('button:has-text("Missions")')).toBeVisible();
    await expect(page.locator('button:has-text("Squadron")')).toBeVisible();

    // Right sidebar tabs
    await expect(page.locator('button:has-text("Ship")')).toBeVisible();
    await expect(page.locator('button:has-text("Comms")')).toBeVisible();
    await expect(page.locator('button:has-text("Combat")')).toBeVisible();
  });

  test('should show footer action buttons on desktop', async ({ page }) => {
    await expect(page.locator('[data-testid="dock-button"]')).toBeVisible();
    await expect(page.locator('[data-testid="stop-button"]')).toBeVisible();
    await expect(page.locator('[data-testid="alert-button"]')).toBeVisible();
  });

  test('should show full resource bar on desktop', async ({ page }) => {
    await expect(page.locator('[data-testid="resource-bar"]')).toBeVisible();
  });

  test('should show logout text on desktop', async ({ page }) => {
    const logoutButton = page.locator('[data-testid="logout-button"]');
    await expect(logoutButton).toContainText('Logout');
  });

  test('should show connection status in footer', async ({ page }) => {
    const footer = page.locator('footer');
    await expect(footer.locator('text=Connected')).toBeVisible();
  });
});
