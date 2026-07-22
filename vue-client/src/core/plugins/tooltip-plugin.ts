import { createApp, inject, ref, type App, type Ref } from 'vue'

export const TOOLTIP_INJECTION_KEY = Symbol('tooltip injection key')
export type ToolTipState = {
  isOpen: Ref<boolean>
  setIsOpen: (newVal: boolean) => void
}

export const toolTipProvider = (): ToolTipState => {
  const isOpen = ref(false)

  const setIsOpen = (newVal: boolean) => {
    isOpen.value = newVal
  }

  return {
    isOpen,
    setIsOpen,
  }
}

export const toolTipPlugin = (app: App) => {
  app.provide<ToolTipState>(TOOLTIP_INJECTION_KEY, toolTipProvider())
}

export const useToolTip = () => {
  const injected = inject<ToolTipState>(TOOLTIP_INJECTION_KEY)
  if (!injected) throw new Error('Could not inject the tooltip state!')
  return injected
}
