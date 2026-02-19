import { create } from 'zustand'

type ConfirmOpts = { title: string; body: string; danger?: boolean; confirmLabel?: string }
type PromptOpts  = { title: string; placeholder?: string; defaultValue?: string }

type ModalState =
  | { kind: 'none' }
  | { kind: 'confirm'; opts: ConfirmOpts; resolve: (v: boolean) => void }
  | { kind: 'prompt';  opts: PromptOpts;  resolve: (v: string | null) => void }

interface ModalStore {
  state: ModalState
  confirm: (opts: ConfirmOpts) => Promise<boolean>
  prompt:  (opts: PromptOpts)  => Promise<string | null>
  accept:  (value?: string)    => void
  cancel:  ()                  => void
}

export const useModalStore = create<ModalStore>((set, get) => ({
  state: { kind: 'none' },

  confirm: (opts) => new Promise<boolean>(resolve =>
    set({ state: { kind: 'confirm', opts, resolve } })
  ),

  prompt: (opts) => new Promise<string | null>(resolve =>
    set({ state: { kind: 'prompt', opts, resolve } })
  ),

  accept: (value) => {
    const s = get().state
    if (s.kind === 'confirm') s.resolve(true)
    if (s.kind === 'prompt')  s.resolve(value ?? null)
    set({ state: { kind: 'none' } })
  },

  cancel: () => {
    const s = get().state
    if (s.kind === 'confirm') s.resolve(false)
    if (s.kind === 'prompt')  s.resolve(null)
    set({ state: { kind: 'none' } })
  },
}))
