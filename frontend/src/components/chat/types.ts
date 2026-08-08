export type ChatStatus =
  | 'initialized'
  | 'connecting'
  | 'connected'
  | 'disconnected'
  | 'suspended'
  | 'closing'
  | 'closed'
  | 'failed'

export interface ChatConnectionFormModel {
  applicationId: string
  channelName: string
  eventName: string
  clientId: string
  displayName: string
}

export interface ChatMember {
  name: string
  typing: boolean
  clientId: string
  connectionId: string
}

export interface ChatMessage {
  id: string
  text: string
  senderId: string
  senderName: string
  sentAt: number
}
