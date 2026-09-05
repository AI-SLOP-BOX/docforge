import { useState, useCallback } from 'react'

export function useHistory(initialData: number[] | null = null, maxHistory = 30) {
  const [data, setData] = useState<number[] | null>(initialData)
  const [history, setHistory] = useState<number[][]>(initialData ? [initialData] : [])
  const [historyIndex, setHistoryIndex] = useState(initialData ? 0 : -1)

  const pushHistory = useCallback((newData: number[]) => {
    setData(newData)
    setHistory(prev => {
      const next = [...prev.slice(0, historyIndex + 1), newData]
      return next.length > maxHistory ? next.slice(-maxHistory) : next
    })
    setHistoryIndex(prev => Math.min(prev + 1, maxHistory - 1))
  }, [historyIndex, maxHistory])

  const undo = useCallback(() => {
    if (historyIndex > 0) {
      const nextIndex = historyIndex - 1
      setHistoryIndex(nextIndex)
      setData(history[nextIndex])
    }
  }, [historyIndex, history])

  const redo = useCallback(() => {
    if (historyIndex < history.length - 1) {
      const nextIndex = historyIndex + 1
      setHistoryIndex(nextIndex)
      setData(history[nextIndex])
    }
  }, [historyIndex, history])

  const resetHistory = useCallback((newData: number[], fileName = '') => {
    setData(newData)
    setHistory([newData])
    setHistoryIndex(0)
  }, [])

  return {
    data,
    setData,
    history,
    historyIndex,
    pushHistory,
    undo,
    redo,
    resetHistory,
    canUndo: historyIndex > 0,
    canRedo: historyIndex >= 0 && historyIndex < history.length - 1,
  }
}
