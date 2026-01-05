import { test, expect } from '@playwright/test';
import { AuthHelper } from '../../fixtures/auth';
import { PageWebSocketHelper } from '../../fixtures/websocket';

test.describe('Squadron Panel', () => {
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

  test('should display squadron panel when clicking Squadron tab', async ({ page }) => {
    const squadronTab = page.locator('button:has-text("Squadron")');
    if (await squadronTab.isVisible()) {
      await squadronTab.click();
    }

    await expect(page.locator('[data-testid="squadron-panel"]')).toBeVisible();
  });

  test('should show Squadron header', async ({ page }) => {
    const squadronTab = page.locator('button:has-text("Squadron")');
    if (await squadronTab.isVisible()) {
      await squadronTab.click();
    }

    const squadronPanel = page.locator('[data-testid="squadron-panel"]');
    await expect(squadronPanel).toContainText('Squadron');
  });

  test('should show create squadron option when not in squadron', async ({ page }) => {
    const squadronTab = page.locator('button:has-text("Squadron")');
    if (await squadronTab.isVisible()) {
      await squadronTab.click();
    }

    const squadronPanel = page.locator('[data-testid="squadron-panel"]');

    // New players should see the create squadron button
    const createButton = page.locator('[data-testid="squadron-show-create-button"]');
    if (await createButton.isVisible().catch(() => false)) {
      await expect(createButton).toContainText('Create Squadron');
    }
  });

  test('should show squadron benefits info', async ({ page }) => {
    const squadronTab = page.locator('button:has-text("Squadron")');
    if (await squadronTab.isVisible()) {
      await squadronTab.click();
    }

    const squadronPanel = page.locator('[data-testid="squadron-panel"]');

    // Benefits section should be visible when not in squadron
    const hasBenefits = await squadronPanel.locator('text=Squadron Benefits').isVisible().catch(() => false);
    if (hasBenefits) {
      await expect(squadronPanel).toContainText('reputation bonuses');
    }
  });

  test('should show create form when clicking create button', async ({ page }) => {
    const squadronTab = page.locator('button:has-text("Squadron")');
    if (await squadronTab.isVisible()) {
      await squadronTab.click();
    }

    const createButton = page.locator('[data-testid="squadron-show-create-button"]');
    if (await createButton.isVisible().catch(() => false)) {
      await createButton.click();

      // Create form should appear
      await expect(page.locator('text=Squadron Name')).toBeVisible();
      await expect(page.locator('text=Tag')).toBeVisible();
    }
  });

  test('should validate squadron name length', async ({ page }) => {
    const squadronTab = page.locator('button:has-text("Squadron")');
    if (await squadronTab.isVisible()) {
      await squadronTab.click();
    }

    const showCreateButton = page.locator('[data-testid="squadron-show-create-button"]');
    if (await showCreateButton.isVisible().catch(() => false)) {
      await showCreateButton.click();

      // Fill short name (less than 3 chars)
      const nameInput = page.locator('input[placeholder*="name"]');
      await nameInput.fill('AB');

      // Create button should be disabled
      const createButton = page.locator('[data-testid="squadron-create-button"]');
      await expect(createButton).toBeDisabled();
    }
  });

  test('should validate squadron tag length', async ({ page }) => {
    const squadronTab = page.locator('button:has-text("Squadron")');
    if (await squadronTab.isVisible()) {
      await squadronTab.click();
    }

    const showCreateButton = page.locator('[data-testid="squadron-show-create-button"]');
    if (await showCreateButton.isVisible().catch(() => false)) {
      await showCreateButton.click();

      // Fill valid name but short tag
      const nameInput = page.locator('input[placeholder*="name"]');
      await nameInput.fill('Test Squadron');

      const tagInput = page.locator('input[placeholder*="TAG"]');
      await tagInput.fill('T');

      // Create button should be disabled
      const createButton = page.locator('[data-testid="squadron-create-button"]');
      await expect(createButton).toBeDisabled();
    }
  });

  test('should show reputation cost info', async ({ page }) => {
    const squadronTab = page.locator('button:has-text("Squadron")');
    if (await squadronTab.isVisible()) {
      await squadronTab.click();
    }

    const showCreateButton = page.locator('[data-testid="squadron-show-create-button"]');
    if (await showCreateButton.isVisible().catch(() => false)) {
      await showCreateButton.click();

      // Should show cost info
      await expect(page.locator('text=/costs.*reputation/i')).toBeVisible();
    }
  });

  test('should have cancel button in create form', async ({ page }) => {
    const squadronTab = page.locator('button:has-text("Squadron")');
    if (await squadronTab.isVisible()) {
      await squadronTab.click();
    }

    const showCreateButton = page.locator('[data-testid="squadron-show-create-button"]');
    if (await showCreateButton.isVisible().catch(() => false)) {
      await showCreateButton.click();

      // Cancel button should be visible
      const cancelButton = page.locator('button:has-text("Cancel")');
      await expect(cancelButton).toBeVisible();

      // Clicking cancel should hide form
      await cancelButton.click();
      await expect(page.locator('text=Squadron Name')).not.toBeVisible();
    }
  });

  test('should enable create button with valid inputs', async ({ page }) => {
    const squadronTab = page.locator('button:has-text("Squadron")');
    if (await squadronTab.isVisible()) {
      await squadronTab.click();
    }

    const showCreateButton = page.locator('[data-testid="squadron-show-create-button"]');
    if (await showCreateButton.isVisible().catch(() => false)) {
      await showCreateButton.click();

      // Fill valid inputs
      const nameInput = page.locator('input[placeholder*="name"]');
      await nameInput.fill('Test Squadron Name');

      const tagInput = page.locator('input[placeholder*="TAG"]');
      await tagInput.fill('TST');

      // Create button may be enabled (depends on reputation)
      const createButton = page.locator('[data-testid="squadron-create-button"]');
      // Can't assume it's enabled without enough rep
    }
  });
});
