import { useApiUrl } from '@/core/plugins/apiUrlPlugin'
import { useContextStore } from '@/core/stores/context-store'
import { storeToRefs } from 'pinia'
import { computed, onBeforeUnmount, onUnmounted, ref, watch } from 'vue'

export function useSerialWebsocket() {
  const { apiUrl } = useApiUrl()

  const contextStore = useContextStore()
  const contextStoreRefs = storeToRefs(contextStore)

  const activeMachineName = ref<string | null>(null)
  const activeSerialPort = ref<string | null>(null)

  const data = ref('')
  const error = ref()
  const wss = ref<WebSocket | null>(null)

  const setActiveMachineAndPort = (machineName: string, port: string) => {
    activeMachineName.value = machineName
    activeSerialPort.value = port
  }

  const activeBaseUrl = computed<string | null>(() => {
    const activeContextId = contextStoreRefs.activeContext.value?.id
    if (!activeContextId || !activeMachineName.value || !activeSerialPort.value) return null

    return `${apiUrl.value}/contexts/${activeContextId}/machines/${activeMachineName.value}/serial/${activeSerialPort.value}`
  })

  const onError = (payload: any) => {
    error.value = payload
  }

  const onMessage = (payload: any) => {
    if (!payload) return
    const decoder = new TextDecoder('utf-8')
    const decoded = decoder.decode(payload)
    if (!decoded) return
    data.value += decoded
  }

  async function clearTerminal() {
    const url = activeBaseUrl.value
    if (!url) throw new Error('Cannot clear terminal with no url')
    await fetch(url, {
      method: 'DELETE',
    })
    data.value = ''
  }

  function buildWebSocket() {
    if (wss.value) {
      unsubscribe()
    }
    if (!activeBaseUrl.value) return

    wss.value = new WebSocket(`${activeBaseUrl.value}/websocket`)
    wss.value.binaryType = 'arraybuffer'

    wss.value.addEventListener('message', ({ data }) => data && onMessage(data))
    wss.value.addEventListener('error', onError)

    // TODO: Why was this written?
    wss.value.addEventListener('open', clearTerminal)
    wss.value.removeEventListener('open', clearTerminal)
  }

  function sendMessage(msg: string) {
    wss.value?.send(msg)
  }

  function unsubscribe() {
    wss.value?.removeEventListener('message', onMessage)
    wss.value?.close()
  }

  watch(activeBaseUrl, () => buildWebSocket(), { immediate: true })

  onBeforeUnmount(() => unsubscribe())

  return {
    data,
    error,
    sendMessage,
    unsubscribe,
    setActiveMachineAndPort,
    wss,
  }
}
