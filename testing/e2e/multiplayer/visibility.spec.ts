import { test, expect } from '@playwright/test';
import { MultiplayerHelper } from '../../fixtures/multiplayer';

test.describe('Sector Visibility', () => {
  let mp: MultiplayerHelper;

  test.afterEach(async () => {
    await mp?.cleanup();
  });

  test('should see other player on sector map', async ({ browser, baseURL }) => {
    mp = new MultiplayerHelper(browser, baseURL!);

    const alice = await mp.createPlayer('alice');
    const bob = await mp.createPlayer('bob');

    // Both should have sector map visible
    await expect(alice.page.locator('[data-testid="sector-map"]')).toBeVisible();
    await expect(bob.page.locator('[data-testid="sector-map"]')).toBeVisible();

    // Wait for state updates
    await alice.page.waitForTimeout(3000);

    const aliceUsername = alice.user.username;
    const bobUsername = bob.user.username;

    // Alice should see Bob's ship on her map
    const bobOnAliceMap = alice.page.locator(`text=${bobUsername}`);
    await expect(bobOnAliceMap).toBeVisible({ timeout: 10000 });

    // Bob should see Alice's ship on his map
    const aliceOnBobMap = bob.page.locator(`text=${aliceUsername}`);
    await expect(aliceOnBobMap).toBeVisible({ timeout: 10000 });
  });

  test('should display player name on ship marker', async ({ browser, baseURL }) => {
    mp = new MultiplayerHelper(browser, baseURL!);

    const alice = await mp.createPlayer('alice');
    const bob = await mp.createPlayer('bob');

    await alice.page.waitForTimeout(3000);

    const bobUsername = bob.user.username;

    // Bob's username should appear on Alice's sector map
    const sectorMap = alice.page.locator('[data-testid="sector-map"]');
    await expect(sectorMap).toBeVisible();

    const bobNameOnMap = sectorMap.locator(`text=${bobUsername}`);
    await expect(bobNameOnMap).toBeVisible({ timeout: 10000 });
  });

  test('should show ship info when clicking player marker', async ({ browser, baseURL }) => {
    mp = new MultiplayerHelper(browser, baseURL!);

    const alice = await mp.createPlayer('alice');
    const bob = await mp.createPlayer('bob');

    await alice.page.waitForTimeout(3000);

    const bobUsername = bob.user.username;

    // Alice clicks on Bob's ship marker
    const bobShipMarker = alice.page.locator(`text=${bobUsername}`).first();
    await expect(bobShipMarker).toBeVisible({ timeout: 10000 });
    await bobShipMarker.click();

    // Should show target info panel with Bob's info
    const targetPanel = alice.page.locator('[data-testid="target-info"], [data-testid="ship-info-panel"]');
    await expect(targetPanel).toBeVisible();
    await expect(targetPanel).toContainText(bobUsername);
  });

  test('should update when player moves', async ({ browser, baseURL }) => {
    mp = new MultiplayerHelper(browser, baseURL!);

    const alice = await mp.createPlayer('alice');
    const bob = await mp.createPlayer('bob');

    await alice.page.waitForTimeout(2000);

    // Verify Bob is visible to Alice
    const bobUsername = bob.user.username;
    const bobMarker = alice.page.locator(`text=${bobUsername}`).first();
    await expect(bobMarker).toBeVisible({ timeout: 10000 });

    // Get Bob's initial position from the position display
    const bobPosition = bob.page.locator('[data-testid="ship-position"]');
    const initialPosition = await bobPosition.textContent();

    // Bob moves by clicking on map
    const bobSectorMap = bob.page.locator('[data-testid="sector-map"]');
    await bobSectorMap.click({ position: { x: 300, y: 300 } });

    // Wait for movement to complete
    await bob.page.waitForTimeout(3000);

    // Bob's position should have changed
    const newPosition = await bobPosition.textContent();
    expect(newPosition).not.toBe(initialPosition);
  });
});
