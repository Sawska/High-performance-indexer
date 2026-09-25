import '@testing-library/jest-dom/vitest'
import { cleanup, configure } from '@testing-library/react'
import { afterEach } from 'vitest'

// Globals are off, so Testing Library cannot register its own cleanup.
afterEach(() => cleanup())

// findBy*/waitFor default to 1s, which a busy machine can exceed.
configure({ asyncUtilTimeout: 5000 })

// jsdom has no ResizeObserver; LineChart only needs it to exist.
if (!('ResizeObserver' in globalThis)) {
  globalThis.ResizeObserver = class {
    observe() {}
    unobserve() {}
    disconnect() {}
  }
}
