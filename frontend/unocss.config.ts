import { extractorSvelte } from '@unocss/core'
import type { Theme } from 'unocss/preset-uno'
import { presetUno, defineConfig as defineConfigUno, presetIcons } from 'unocss'
import transformerDirectives from '@unocss/transformer-directives'
 
export default defineConfigUno<Theme>({
  theme: {
    colors: {
      background: {
        "50": "#131314",
        "100": "#161617",
        "150": "#18181a",
        "200": "#1b1b1d"
      },
      primary: "#ff1d23"
    }
  },
  presets: [
    presetUno(),
    presetIcons({
    }),
  ],
  extractors: [extractorSvelte],
  transformers: [
    transformerDirectives()
  ]
});