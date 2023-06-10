// @unocss-include
import { defineConfig, toEscapedSelector } from 'unocss'
import presetUno from '@unocss/preset-uno'
import transformerDirectives from '@unocss/transformer-directives';

export default defineConfig({
    presets: [
        presetUno({
            dark: 'class',
        }),
    ],
    transformers: [
        transformerDirectives(),
    ],
    shortcuts: {
        "text": "text-gray-8 dark:text-gray-1",
        "bg": "bg-white dark:bg-dark-8"
    },
    rules: [
        [/^sel-(\w*)(-|:)(.*)$/, async ([, sel, , r], {rawSelector, generator}) => { 
            const rule = await (generator.parseToken(r))
            // @ts-expect-error
            return `${toEscapedSelector(rawSelector)} ${sel} { ${rule[0][2]} }`
        }],
      ],
    preflights: [
        {
          getCSS: () => `
            .btn i {
                @apply w-5 h-5;
            }
            .btn-icon svg {
                @apply w-6 h-6;
            }
          `
        }
      ]
    
})