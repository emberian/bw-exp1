import { test, expect } from '@playwright/test';
import { AuthHelper } from '../../fixtures/auth';

test.describe('Logout Flow', () => {
  let auth: AuthHelper;

  test.beforeEach(async ({ page, baseURL }) => {
    auth = new AuthHelper(page, baseURL!);
  });

  test('should clear session on logout via API', async ({ page }) => {
    // Create and login user
    const user = await auth.createAndLogin();

    // Verify we have a token
    expect(user.token).toBeTruthy();

    // Validate token is valid
    const isValid = await auth.validateToken(user.token!);
    expect(isValid).toBe(true);

    // Logout via API
    await auth.logoutViaAPI(user.token!);

    // Token should now be invalid
    const isValidAfterLogout = await auth.validateToken(user.token!);
    expect(isValidAfterLogout).toBe(false);
  });

  test('should redirect to login after clearing token', async ({ page }) => {
    // Create and login user
    await auth.createAndLogin();

    // Go to game page
    await page.goto('/play/game');
    await expect(page).toHaveURL(/\/play\/game/);

    // Clear token (simulating logout)
    await auth.clearToken();

    // Refresh page
    await page.reload();

    // Should redirect to login/register
    await expect(page).toHaveURL(/\/play\/(register|login)/, { timeout: 10000 });
  });

  test('should not be able to access game after logout', async ({ page }) => {
    // Create and login user
    const user = await auth.createAndLogin();

    // Verify we can access game
    await page.goto('/play/game');
    await expect(page).toHaveURL(/\/play\/game/);

    // Logout via API and clear token
    await auth.logoutViaAPI(user.token!);
    await auth.clearToken();

    // Try to access game again
    await page.goto('/play/game');

    // Should redirect to login/register
    await expect(page).toHaveURL(/\/play\/(register|login)/, { timeout: 10000 });
  });
});
