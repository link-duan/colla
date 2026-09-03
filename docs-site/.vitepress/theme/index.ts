import DefaultTheme from 'vitepress/theme'
import './custom.css'
import OtWorkbench from './components/OtWorkbench.vue'

export default {
  extends: DefaultTheme,
  enhanceApp({ app }: { app: any }) {
    app.component('OtWorkbench', OtWorkbench)
  }
}
