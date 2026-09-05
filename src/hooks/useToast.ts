import { useState, useCallback } from 'react'

export function useToast(duration = 2800) {
  const [toast, setToast] = useState<string | null>(null)
  const [toastType, setToastType] = useState<'info' | 'error' | 'success'>('info')

  const showToast = useCallback((msg: string, type: 'info' | 'error' | 'success' = 'info') => {
    setToast(msg)
    setToastType(type)
    setTimeout(() => {
      setToast(null)
    }, duration)
  }, [duration])

  const showError = useCallback((msg: string) => {
    showToast(msg, 'error')
  }, [showToast])

  const showSuccess = useCallback((msg: string) => {
    showToast(msg, 'success')
  }, [showToast])

  return {
    toast,
    toastType,
    showToast,
    showError,
    showSuccess,
  }
}
