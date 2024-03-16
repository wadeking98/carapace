// Utilities
import { defineStore } from 'pinia'

interface State{
  messages: Message
}
interface Message {
  activeContact: string | undefined
  contactDraftMessages: Record<string, string>
}

export const useAppStore = defineStore('app', {
  state: (): State => ({
    messages:{
      activeContact: undefined,
      contactDraftMessages: {}
    }
  }),
})
