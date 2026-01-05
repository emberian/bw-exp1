import { test, expect } from '@playwright/test';
import { MultiplayerHelper } from '../../fixtures/multiplayer';

test.describe('Multiplayer Chat', () => {
  let mp: MultiplayerHelper;

  test.afterEach(async () => {
    await mp?.cleanup();
  });

  test('should deliver sector chat message to other player', async ({ browser, baseURL }) => {
    mp = new MultiplayerHelper(browser, baseURL!);

    const alice = await mp.createPlayer('alice');
    const bob = await mp.createPlayer('bob');

    // Both open comms panel
    await alice.page.locator('button:has-text("Comms")').click();
    await bob.page.locator('button:has-text("Comms")').click();

    await expect(alice.page.locator('[data-testid="comms-panel"]')).toBeVisible();
    await expect(bob.page.locator('[data-testid="comms-panel"]')).toBeVisible();

    // Generate unique message
    const uniqueMessage = `Hello from Alice ${Date.now()}`;

    // Alice sends a sector chat message
    await alice.page.locator('[data-testid="chat-input"]').fill(uniqueMessage);
    await alice.page.locator('[data-testid="chat-send-button"]').click();

    // Bob should receive the message
    await expect(bob.page.locator('[data-testid="chat-message"]').last())
      .toContainText(uniqueMessage, { timeout: 10000 });
  });

  test('should show sender name with message', async ({ browser, baseURL }) => {
    mp = new MultiplayerHelper(browser, baseURL!);

    const alice = await mp.createPlayer('alice');
    const bob = await mp.createPlayer('bob');

    // Both open comms panel
    await alice.page.locator('button:has-text("Comms")').click();
    await bob.page.locator('button:has-text("Comms")').click();

    await expect(alice.page.locator('[data-testid="comms-panel"]')).toBeVisible();
    await expect(bob.page.locator('[data-testid="comms-panel"]')).toBeVisible();

    const uniqueMessage = `Test message ${Date.now()}`;
    const aliceUsername = alice.user.username;

    // Alice sends message
    await alice.page.locator('[data-testid="chat-input"]').fill(uniqueMessage);
    await alice.page.locator('[data-testid="chat-send-button"]').click();

    // Bob should see message with Alice's username
    const messageContainer = bob.page.locator('[data-testid="chat-message"]').last();
    await expect(messageContainer).toContainText(uniqueMessage, { timeout: 10000 });
    await expect(messageContainer).toContainText(aliceUsername);
  });

  test('should deliver squadron chat to squadron members', async ({ browser, baseURL }) => {
    mp = new MultiplayerHelper(browser, baseURL!);

    const alice = await mp.createPlayer('alice');
    const bob = await mp.createPlayer('bob');

    // Alice creates squadron and invites Bob
    await alice.page.locator('button:has-text("Squadron")').click();
    const createButton = alice.page.locator('[data-testid="squadron-show-create-button"]');
    await expect(createButton).toBeVisible();
    await createButton.click();

    await alice.page.locator('input[placeholder*="name" i]').fill('ChatSquad');
    await alice.page.locator('input[placeholder*="tag" i]').fill('CSQ');
    await alice.page.locator('[data-testid="squadron-create-button"]').click();

    // Alice invites Bob
    const bobUsername = bob.user.username;
    await alice.page.waitForTimeout(2000);

    const bobShipMarker = alice.page.locator(`text=${bobUsername}`).first();
    await expect(bobShipMarker).toBeVisible({ timeout: 10000 });
    await bobShipMarker.click();

    const inviteButton = alice.page.locator('button:has-text(/invite.*squadron|squadron.*invite/i)');
    await expect(inviteButton).toBeVisible();
    await inviteButton.click();

    // Bob accepts
    const bobNotification = bob.page.locator('[data-testid="notifications-panel"]');
    await expect(bobNotification).toBeVisible({ timeout: 10000 });
    await bob.page.locator('button:has-text("Accept")').first().click();

    // Both open comms panel
    await alice.page.locator('button:has-text("Comms")').click();
    await bob.page.locator('button:has-text("Comms")').click();

    // Switch to squadron channel
    await alice.page.locator('button:has-text("Squad")').click();
    await bob.page.locator('button:has-text("Squad")').click();

    const squadMessage = `Squad message ${Date.now()}`;
    await alice.page.locator('[data-testid="chat-input"]').fill(squadMessage);
    await alice.page.locator('[data-testid="chat-send-button"]').click();

    // Bob should receive squadron message
    await expect(bob.page.locator('[data-testid="chat-message"]').last())
      .toContainText(squadMessage, { timeout: 10000 });
  });

  test('should not deliver squadron chat to non-members', async ({ browser, baseURL }) => {
    mp = new MultiplayerHelper(browser, baseURL!);

    const alice = await mp.createPlayer('alice');
    const bob = await mp.createPlayer('bob');
    const charlie = await mp.createPlayer('charlie');

    // Alice creates squadron (does NOT invite Bob or Charlie)
    await alice.page.locator('button:has-text("Squadron")').click();
    const createButton = alice.page.locator('[data-testid="squadron-show-create-button"]');
    await expect(createButton).toBeVisible();
    await createButton.click();

    await alice.page.locator('input[placeholder*="name" i]').fill('PrivateSquad');
    await alice.page.locator('input[placeholder*="tag" i]').fill('PRV');
    await alice.page.locator('[data-testid="squadron-create-button"]').click();

    // All open comms panel
    await alice.page.locator('button:has-text("Comms")').click();
    await bob.page.locator('button:has-text("Comms")').click();
    await charlie.page.locator('button:has-text("Comms")').click();

    // Alice switches to squadron channel and sends message
    await alice.page.locator('button:has-text("Squad")').click();

    const privateMessage = `Private squad message ${Date.now()}`;
    await alice.page.locator('[data-testid="chat-input"]').fill(privateMessage);
    await alice.page.locator('[data-testid="chat-send-button"]').click();

    // Wait for message propagation
    await bob.page.waitForTimeout(3000);

    // Bob (non-member) should NOT see the squadron message
    const bobMessages = await bob.page.locator('[data-testid="chat-message"]').allTextContents();
    const bobSawPrivate = bobMessages.some(m => m.includes(privateMessage));
    expect(bobSawPrivate).toBe(false);

    // Charlie (non-member) should also NOT see it
    const charlieMessages = await charlie.page.locator('[data-testid="chat-message"]').allTextContents();
    const charlieSawPrivate = charlieMessages.some(m => m.includes(privateMessage));
    expect(charlieSawPrivate).toBe(false);
  });
});
