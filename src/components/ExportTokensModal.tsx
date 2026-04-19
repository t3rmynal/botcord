import { useEffect, useState } from 'react'
import { AnimatePresence, motion } from 'framer-motion'
import { save as saveDialog } from '@tauri-apps/plugin-dialog'
import { api } from '../lib/ipc'

export function ExportTokensModal({
  open,
  onClose,
  ids,
}: {
  open: boolean
  onClose: () => void
  ids: string[] | null
}) {
  const [tokens, setTokens] = useState<string[]>([])
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [copied, setCopied] = useState(false)
  const [savedTo, setSavedTo] = useState<string | null>(null)

  useEffect(() => {
    if (!open) {
      setTokens([])
      setCopied(false)
      setSavedTo(null)
      setError(null)
      return
    }
    setLoading(true)
    api
      .accountsExportTokens(ids)
      .then((r) => setTokens(r))
      .catch((e) => setError(String(e)))
      .finally(() => setLoading(false))
  }, [open, ids])

  const text = tokens.join('\n')

  const copy = async () => {
    if (tokens.length === 0) return
    await navigator.clipboard.writeText(text)
    setCopied(true)
    setTimeout(() => setCopied(false), 1200)
  }

  const saveFile = async () => {
    const stamp = new Date().toISOString().slice(0, 10)
    const path = await saveDialog({
      defaultPath: `tokens-${stamp}.txt`,
      filters: [{ name: 'text', extensions: ['txt'] }],
    })
    if (typeof path !== 'string') return
    try {
      await api.saveTextFile(path, text)
      setSavedTo(path)
      setTimeout(() => setSavedTo(null), 3000)
    } catch (e) {
      setError(String(e))
    }
  }

  return (
    <AnimatePresence>
      {open && (
        <motion.div
          className="fixed inset-0 bg-black/75 backdrop-blur-sm grid place-items-center z-50 p-6"
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          exit={{ opacity: 0 }}
          onClick={onClose}
        >
          <motion.div
            onClick={(e) => e.stopPropagation()}
            initial={{ y: 16, opacity: 0 }}
            animate={{ y: 0, opacity: 1 }}
            exit={{ y: 8, opacity: 0 }}
            transition={{ type: 'spring', stiffness: 300, damping: 28 }}
            className="w-[640px] max-w-full border border-line-3 bg-bg overflow-hidden flex flex-col"
          >
            <div className="h-10 px-4 flex items-center gap-3 border-b border-line">
              <span className="text-[11px] uppercase tracking-[0.22em]">export tokens</span>
              <span className="ml-auto text-[10px] text-ink-3">
                {loading ? '...' : `${tokens.length} token${tokens.length === 1 ? '' : 's'}`}
              </span>
              <button
                className="h-7 px-2 text-[10px] uppercase tracking-[0.22em] text-ink-3 hover:text-ink transition"
                onClick={onClose}
              >
                esc
              </button>
            </div>
            <div className="p-4 flex flex-col gap-3">
              <textarea
                value={text}
                readOnly
                spellCheck={false}
                className="min-h-[180px] max-h-[320px] p-3 bg-bg-2 border border-line-2 outline-none text-[12px] font-mono resize-y"
              />
              {error && (
                <div className="text-[11px] text-bad border-l-2 border-bad pl-2 font-mono">
                  {error}
                </div>
              )}
              <div className="flex items-center gap-2">
                <button
                  onClick={copy}
                  disabled={tokens.length === 0}
                  className="h-8 px-4 border border-line-3 hover:border-ink text-[11px] uppercase tracking-[0.22em] disabled:opacity-30 transition"
                >
                  {copied ? 'copied' : 'copy all'}
                </button>
                <button
                  onClick={saveFile}
                  disabled={tokens.length === 0}
                  className="h-8 px-4 border border-line-3 hover:border-ink text-[11px] uppercase tracking-[0.22em] disabled:opacity-30 transition"
                >
                  save to file
                </button>
                {savedTo && (
                  <span className="text-[10px] uppercase tracking-[0.22em] text-ok truncate">
                    saved · {savedTo}
                  </span>
                )}
              </div>
            </div>
          </motion.div>
        </motion.div>
      )}
    </AnimatePresence>
  )
}
