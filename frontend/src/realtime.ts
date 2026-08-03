import { Realtime } from 'ably'
import type {
  ConnectionStateChange,
  InboundMessage,
  PresenceMessage,
  RealtimeChannel,
} from 'ably'

export interface ChatPayload {
  id: string
  text: string
  senderId: string
  senderName: string
  sentAt: number
}

export interface PresencePayload {
  name: string
  typing: boolean
}

interface RealtimeChatOptions {
  applicationId: string
  channelName: string
  eventName: string
  clientId: string
  displayName: string
  onConnectionChange: (change: ConnectionStateChange) => void
  onMessage: (message: InboundMessage) => void
  onPresence: (message: PresenceMessage) => void
}

export class RealtimeChat {
  readonly client: Realtime

  private readonly channel: RealtimeChannel
  private readonly eventName: string
  private readonly displayName: string
  private readonly onMessage: (message: InboundMessage) => void
  private readonly onPresence: (message: PresenceMessage) => void

  constructor({
    applicationId,
    channelName,
    eventName,
    clientId,
    displayName,
    onConnectionChange,
    onMessage,
    onPresence,
  }: RealtimeChatOptions) {
    const endpoint = import.meta.env.DEV
      ? { endpoint: '127.0.0.1', port: 4008, tls: false }
      : { endpoint: location.hostname }

    this.eventName = eventName
    this.displayName = displayName
    this.onMessage = onMessage
    this.onPresence = onPresence

    this.client = new Realtime({
      ...endpoint,
      clientId,
      transports: ['web_socket'],
      fallbackHosts: [],
      disableConnectivityCheck: true,
      useBinaryProtocol: false,
      queueMessages: false,
      authCallback: (_, done) => {
        issueAccessToken(applicationId, clientId).then(
          (token) => done(null, token),
          (error) => done(String(error), null),
        )
      },
    })

    this.channel = this.client.channels.get(channelName)
    this.client.connection.on(onConnectionChange)
  }

  async connect(): Promise<void> {
    await Promise.all([
      this.channel.subscribe(this.onMessage),
      this.channel.presence.subscribe(this.onPresence),
    ])
    await this.channel.presence.enter(this.presence(false))
  }

  publish(payload: ChatPayload) {
    return this.channel.publish(this.eventName, JSON.stringify(payload))
  }

  setTyping(typing: boolean): Promise<void> {
    return this.channel.presence.update(this.presence(typing))
  }

  async close(): Promise<void> {
    await this.channel.presence.leave(this.presence(false))
    this.channel.unsubscribe()
    this.channel.presence.unsubscribe()
    this.client.close()
  }

  private presence(typing: boolean): string {
    return JSON.stringify({ name: this.displayName, typing })
  }
}

export function chatPayload(message: InboundMessage): ChatPayload {
  const data = message.data
  const payload = typeof data === 'string' && data.startsWith('{')
    ? JSON.parse(data) as ChatPayload
    : null

  return payload ?? {
    id: message.id ?? crypto.randomUUID(),
    text: typeof data === 'string' ? data : JSON.stringify(data ?? null),
    senderId: message.clientId ?? '',
    senderName: message.clientId ?? message.name ?? 'REST API',
    sentAt: message.timestamp ?? Date.now(),
  }
}

export const presencePayload = ({ data }: PresenceMessage): PresencePayload =>
  JSON.parse(data as string) as PresencePayload

async function issueAccessToken(applicationId: string, clientId: string): Promise<string> {
  const response = await fetch(`/auth/realtime/${encodeURIComponent(applicationId)}/token`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ clientId }),
  })

  const { data } = await response.json() as { data: { jwt: string } }

  return data.jwt
}
