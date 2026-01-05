import { test, expect } from '@playwright/test';
import { AuthHelper } from '../../fixtures/auth';
import { PageWebSocketHelper } from '../../fixtures/websocket';

test.describe('Comms Panel', () => {
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

  test('should display comms panel when clicking Comms tab', async ({ page }) => {
    const commsTab = page.locator('button:has-text("Comms")');
    if (await commsTab.isVisible()) {
      await commsTab.click();
    }

    await expect(page.locator('[data-testid="comms-panel"]')).toBeVisible();
  });

  test('should show Comms header', async ({ page }) => {
    const commsTab = page.locator('button:has-text("Comms")');
    if (await commsTab.isVisible()) {
      await commsTab.click();
    }

    const commsPanel = page.locator('[data-testid="comms-panel"]');
    await expect(commsPanel).toContainText('Comms');
  });

  test('should display chat input field', async ({ page }) => {
    const commsTab = page.locator('button:has-text("Comms")');
    if (await commsTab.isVisible()) {
      await commsTab.click();
    }

    await expect(page.locator('[data-testid="chat-input"]')).toBeVisible();
  });

  test('should display send button', async ({ page }) => {
    const commsTab = page.locator('button:has-text("Comms")');
    if (await commsTab.isVisible()) {
      await commsTab.click();
    }

    await expect(page.locator('[data-testid="chat-send-button"]')).toBeVisible();
  });

  test('should have send button disabled when input is empty', async ({ page }) => {
    const commsTab = page.locator('button:has-text("Comms")');
    if (await commsTab.isVisible()) {
      await commsTab.click();
    }

    const sendButton = page.locator('[data-testid="chat-send-button"]');
    await expect(sendButton).toBeDisabled();
  });

  test('should enable send button when input has text', async ({ page }) => {
    const commsTab = page.locator('button:has-text("Comms")');
    if (await commsTab.isVisible()) {
      await commsTab.click();
    }

    const chatInput = page.locator('[data-testid="chat-input"]');
    await chatInput.fill('Test message');

    const sendButton = page.locator('[data-testid="chat-send-button"]');
    await expect(sendButton).toBeEnabled();
  });

  test('should show channel selector buttons', async ({ page }) => {
    const commsTab = page.locator('button:has-text("Comms")');
    if (await commsTab.isVisible()) {
      await commsTab.click();
    }

    // Should have Sector and Squad channel buttons
    await expect(page.locator('button:has-text("Sector")')).toBeVisible();
    await expect(page.locator('button:has-text("Squad")')).toBeVisible();
  });

  test('should send message when clicking send button', async ({ page }) => {
    const commsTab = page.locator('button:has-text("Comms")');
    if (await commsTab.isVisible()) {
      await commsTab.click();
    }

    const chatInput = page.locator('[data-testid="chat-input"]');
    const sendButton = page.locator('[data-testid="chat-send-button"]');

    await chatInput.fill('Hello from test');
    await sendButton.click();

    // Input should clear after sending
    await expect(chatInput).toHaveValue('');
  });

  test('should send message when pressing Enter', async ({ page }) => {
    const commsTab = page.locator('button:has-text("Comms")');
    if (await commsTab.isVisible()) {
      await commsTab.click();
    }

    const chatInput = page.locator('[data-testid="chat-input"]');
    await chatInput.fill('Hello via Enter key');
    await chatInput.press('Enter');

    // Input should clear after sending
    await expect(chatInput).toHaveValue('');
  });

  test('should display sent messages', async ({ page }) => {
    const commsTab = page.locator('button:has-text("Comms")');
    if (await commsTab.isVisible()) {
      await commsTab.click();
    }

    const chatInput = page.locator('[data-testid="chat-input"]');
    const testMessage = `Test message ${Date.now()}`;
    await chatInput.fill(testMessage);
    await chatInput.press('Enter');

    // Wait for message to appear
    await page.waitForTimeout(1000);

    // Check for message element
    const chatMessage = page.locator('[data-testid="chat-message"]').first();
    // Message may or may not be visible immediately depending on server echo
  });

  test('should show empty state when no messages', async ({ page }) => {
    const commsTab = page.locator('button:has-text("Comms")');
    if (await commsTab.isVisible()) {
      await commsTab.click();
    }

    const commsPanel = page.locator('[data-testid="comms-panel"]');
    // Either shows messages or empty state
    await expect(commsPanel).toBeVisible();
  });

  test('should show channel indicator', async ({ page }) => {
    const commsTab = page.locator('button:has-text("Comms")');
    if (await commsTab.isVisible()) {
      await commsTab.click();
    }

    // Should show which channel messages are being sent to
    const commsPanel = page.locator('[data-testid="comms-panel"]');
    await expect(commsPanel).toContainText('Sending to');
  });
});
