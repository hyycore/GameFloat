import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import Settings from './Settings'
import './settings.css'

createRoot(document.getElementById('root') as HTMLElement).render(
  <StrictMode>
    <Settings />
  </StrictMode>
)
