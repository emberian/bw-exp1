import { Page } from '@playwright/test';
import { encode, decode } from '@msgpack/msgpack';

/**
 * WebSocket connection state (mirrors frontend ConnectionState)
 */
export enum ConnectionState {
  Disconnected = 'Disconnected',
  Connecting = 'Connecting',
  Connected = 'Connected',
  Reconnecting = 'Reconnecting',
}

/**
 * Client message types (subset of bw_shared::messages::ClientMessage)
 */
export interface ClientMessage {
  Authenticate?: { token: string };
  MoveToPosition?: { x: number; y: number; z: number };
  StopMovement?: Record<string, never>;
  DockAtStation?: { station_id: string };
  Undock?: Record<string, never>;
  AcceptMission?: { mission_id: string };
  AbandonMission?: { mission_id: string };
  MakeMissionChoice?: { mission_id: string; choice_id: string };
  EngageCombat?: { target_id: string };
  DisengageCombat?: Record<string, never>;
  SendChat?: { channel: string; message: string };
  Ping?: { client_time: number };
}

/**
 * Server message types (subset of bw_shared::messages::ServerMessage)
 */
export interface ServerMessage {
  AuthResult?: { success: boolean; error?: string };
  InitialState?: unknown;
  StateUpdate?: unknown;
  MissionChoice?: unknown;
  MissionResult?: unknown;
  ChatMessage?: unknown;
  Pong?: { client_time: number; server_time: number };
  Error?: { message: string };
}

/**
 * Direct WebSocket helper for Node.js environment (fixtures/global setup)
 */
export class WebSocketClient {
  private ws: WebSocket | null = null;
  private messageQueue: ServerMessage[] = [];

  constructor(private wsUrl: string) {}

  async connect(token: string): Promise<void> {
    return new Promise((resolve, reject) => {
      this.ws = new WebSocket(this.wsUrl);

      this.ws.onopen = () => {
        // Send authentication
        const authMsg: ClientMessage = { Authenticate: { token } };
        this.ws!.send(encode(authMsg));
      };

      this.ws.onmessage = async (event) => {
        const data = event.data instanceof Blob
          ? await event.data.arrayBuffer()
          : event.data;
        const msg = decode(new Uint8Array(data)) as ServerMessage;
        this.messageQueue.push(msg);

        // Resolve on successful auth
        if (msg.AuthResult?.success) {
          resolve();
        } else if (msg.AuthResult && !msg.AuthResult.success) {
          reject(new Error(`Auth failed: ${msg.AuthResult.error}`));
        }
      };

      this.ws.onerror = (error) => {
        reject(error);
      };

      setTimeout(() => reject(new Error('WebSocket connection timeout')), 10000);
    });
  }

  send(message: ClientMessage): void {
    if (!this.ws || this.ws.readyState !== WebSocket.OPEN) {
      throw new Error('WebSocket not connected');
    }
    this.ws.send(encode(message));
  }

  async waitForMessage(
    predicate: (msg: ServerMessage) => boolean,
    timeout = 5000
  ): Promise<ServerMessage> {
    return new Promise((resolve, reject) => {
      const timer = setTimeout(() => {
        reject(new Error('Timeout waiting for message'));
      }, timeout);

      // Check existing queue
      const existing = this.messageQueue.find(predicate);
      if (existing) {
        clearTimeout(timer);
        resolve(existing);
        return;
      }

      // Listen for new messages
      const originalOnMessage = this.ws?.onmessage;
      this.ws!.onmessage = async (event) => {
        const data = event.data instanceof Blob
          ? await event.data.arrayBuffer()
          : event.data;
        const msg = decode(new Uint8Array(data)) as ServerMessage;
        this.messageQueue.push(msg);

        if (predicate(msg)) {
          clearTimeout(timer);
          this.ws!.onmessage = originalOnMessage;
          resolve(msg);
        }
      };
    });
  }

  getMessages(): ServerMessage[] {
    return [...this.messageQueue];
  }

  clearMessages(): void {
    this.messageQueue = [];
  }

  disconnect(): void {
    if (this.ws) {
      this.ws.close();
      this.ws = null;
    }
  }
}

/**
 * Page-based WebSocket helper for browser context tests
 */
export class PageWebSocketHelper {
  constructor(private page: Page) {}

  /**
   * Wait for WebSocket connection indicator to show connected state
   */
  async waitForConnection(timeout = 15000): Promise<void> {
    // Wait for connected state indicator (green dot or "Connected" text)
    await this.page.waitForFunction(
      () => {
        // Check for green connection indicator
        const greenIndicator = document.querySelector('.bg-green-500, .bg-green-400');
        const connectedText = document.body.innerText.includes('Connected');
        return greenIndicator !== null || connectedText;
      },
      { timeout }
    );
  }

  /**
   * Wait for initial game state to load (sector map visible)
   */
  async waitForInitialState(timeout = 20000): Promise<void> {
    await this.page.waitForSelector('[data-testid="sector-map"], canvas, svg', { timeout });
  }

  /**
   * Get current connection state from page
   */
  async getConnectionState(): Promise<string> {
    return await this.page.evaluate(() => {
      const text = document.body.innerText;
      if (text.includes('Connected')) return 'Connected';
      if (text.includes('Reconnecting')) return 'Reconnecting';
      if (text.includes('Connecting')) return 'Connecting';
      return 'Disconnected';
    });
  }

  /**
   * Simulate WebSocket disconnect for testing reconnection
   */
  async simulateDisconnect(): Promise<void> {
    await this.page.evaluate(() => {
      // Access WebSocket through window and close it
      const ws = (window as unknown as { __wsInstance?: WebSocket }).__wsInstance;
      if (ws) {
        ws.close();
      }
    });
  }
}
