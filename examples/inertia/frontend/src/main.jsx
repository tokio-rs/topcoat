import { createInertiaApp } from '@inertiajs/react'
import { createRoot } from 'react-dom/client'
import './style.css'

const pages = import.meta.glob('./pages/**/*.jsx', { eager: true })

createInertiaApp({
  id: 'inertia-app',
  resolve: (name) => pages[`./pages/${name}.jsx`].default,
  setup({ el, App, props }) {
    createRoot(el).render(<App {...props} />)
  },
})
