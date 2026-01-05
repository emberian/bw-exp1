import { test, expect } from '@playwright/test';
import { AuthHelper } from '../../fixtures/auth';

test.describe('Login Flow', () => {
  let auth: AuthHelper;

  test.beforeEach(async ({ page, baseURL }) => {
    auth = new AuthHelper(page, baseURL!);
  });

  test('should display login page correctly', async ({ page }) => {
    await page.goto('/play/login');

    // Check page title
    await expect(page.locator('h1')).toContainText('Welcome Back');

    // Check form elements
    await expect(page.locator('[data-testid="username-input"]')).toBeVisible();
    await expect(page.locator('[data-testid="password-input"]')).toBeVisible();
    await expect(page.locator('[data-testid="submit-button"]')).toBeVisible();

    // Check navigation links
    await expect(page.locator('a[href="/play/register"]')).toBeVisible();
  });

  test('should show error for empty username', async ({ page }) => {
    await page.goto('/play/login');

    // Submit with empty username
    await page.fill('[data-testid="password-input"]', 'somepassword');
    await page.click('[data-testid="submit-button"]');

    // Check for error message
    await expect(page.locator('[data-testid="error-message"]')).toContainText('Username is required');
  });

  test('should show error for empty password', async ({ page }) => {
    await page.goto('/play/login');

    // Submit with empty password
    await page.fill('[data-testid="username-input"]', 'someuser');
    await page.click('[data-testid="submit-button"]');

    // Check for error message
    await expect(page.locator('[data-testid="error-message"]')).toContainText('Password is required');
  });

  test('should show error for invalid credentials', async ({ page }) => {
    await page.goto('/play/login');

    // Submit with invalid credentials
    await page.fill('[data-testid="username-input"]', 'nonexistent_user');
    await page.fill('[data-testid="password-input"]', 'wrongpassword123');
    await page.click('[data-testid="submit-button"]');

    // Check for error message
    await expect(page.locator('[data-testid="error-message"]')).toContainText('Invalid username or password');
  });

  test('should login successfully with valid credentials', async ({ page }) => {
    // First register a user via API
    const user = await auth.registerUser({
      username: `test_login_${Date.now()}`,
      password: 'validpassword123',
    });

    await page.goto('/play/login');

    // Fill in valid credentials
    await page.fill('[data-testid="username-input"]', user.username);
    await page.fill('[data-testid="password-input"]', user.password);
    await page.click('[data-testid="submit-button"]');

    // Should redirect to game page
    await expect(page).toHaveURL(/\/play\/game/, { timeout: 15000 });
  });

  test('should persist session in localStorage after login', async ({ page }) => {
    // Register a user
    const user = await auth.registerUser();

    await page.goto('/play/login');

    // Login
    await page.fill('[data-testid="username-input"]', user.username);
    await page.fill('[data-testid="password-input"]', user.password);
    await page.click('[data-testid="submit-button"]');

    // Wait for redirect
    await expect(page).toHaveURL(/\/play\/game/, { timeout: 15000 });

    // Check localStorage has token
    const token = await auth.getToken();
    expect(token).toBeTruthy();
  });

  test('should maintain session after page refresh', async ({ page }) => {
    // Create and login user via API
    const user = await auth.createAndLogin();

    // Go to game page
    await page.goto('/play/game');
    await expect(page).toHaveURL(/\/play\/game/);

    // Refresh the page
    await page.reload();

    // Should still be on game page (not redirected to login)
    await expect(page).toHaveURL(/\/play\/game/);
  });

  test('should redirect unauthenticated users from game to login', async ({ page }) => {
    // Clear any existing token
    await page.goto('/play/login');
    await auth.clearToken();

    // Try to access game page
    await page.goto('/play/game');

    // Should redirect to register page (since no auth)
    await expect(page).toHaveURL(/\/play\/(register|login)/, { timeout: 10000 });
  });

  test('should show loading state while logging in', async ({ page }) => {
    const user = await auth.registerUser();

    await page.goto('/play/login');
    await page.fill('[data-testid="username-input"]', user.username);
    await page.fill('[data-testid="password-input"]', user.password);

    // Click and immediately check for loading state
    const submitButton = page.locator('[data-testid="submit-button"]');
    await submitButton.click();

    // Button should show loading text (may be brief)
    // We use a soft assertion since it might be too fast to catch
    await expect(submitButton).toBeDisabled();
  });

  test('should navigate to register page from login', async ({ page }) => {
    await page.goto('/play/login');

    // Click register link
    await page.click('a[href="/play/register"]');

    // Should be on register page
    await expect(page).toHaveURL('/play/register');
  });
});
