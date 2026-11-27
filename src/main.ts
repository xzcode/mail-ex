import { createApp } from 'vue'
import { getCurrentWindow } from '@tauri-apps/api/window'
import App from './App.vue'
import ReaderWindow from './components/ReaderWindow.vue'
import './style.css'

let label='main'
try{label=getCurrentWindow().label}catch{/* browser development preview */}
createApp(label.startsWith('reader-')?ReaderWindow:App).mount('#app')
