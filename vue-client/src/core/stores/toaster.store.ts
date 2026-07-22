import { defineStore } from 'pinia'
import {
  alertBuilder,
  type ToasterAlert,
  type ToasterAlertDisplay,
  type ToasterAlertOptions,
} from '@/shared/types/toaster.model'
import { ref } from 'vue'

interface ToasterState {
  activeAlert: ToasterAlert | null
  alertQueue: ToasterAlert[]
}

export const useToasterStore = defineStore('toaster', () => {
  const queue = ref<ToasterAlertDisplay[]>([])

  const activeAlert = ref<ToasterAlertDisplay | null>(null)

  let timerRef: ReturnType<typeof setTimeout> | null = null

  function createAlert(title: string, options: ToasterAlertOptions) {
    const newAlert = alertBuilder(title, options)

    queue.value.push(newAlert)

    nextAlert()
  }

  function nextAlert() {
    if (queue.value.length === 0) {
      activeAlert.value = null
      return
    }

    const next = queue.value.shift()

    if (!next) return

    activeAlert.value = next

    timerRef = setTimeout(() => {
      activeAlert.value = null
      timerRef = null
      nextAlert()
    }, next.durationTime)
  }

  return {
    activeAlert,
    createAlert,
  }
})
