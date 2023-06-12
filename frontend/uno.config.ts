// @unocss-include
import { defineConfig, toEscapedSelector } from 'unocss'
import presetUno from '@unocss/preset-uno'
import transformerVariantGroup from '@unocss/transformer-variant-group';
import transformerDirectives from '@unocss/transformer-directives';

export default defineConfig({
    presets: [
        presetUno({
            dark: 'class',
        }),
    ],
    transformers: [
        transformerVariantGroup(),
        transformerDirectives(),
    ],
    shortcuts: {
        "background-body": "bg-light-50 dark:bg-dark-8",
        "text-color": "text-gray-8 dark:text-gray-2",
        "button-transparent": "bg-transparent p2 rounded-lg text-color enabled:hover:(bg-gray-500/20 dark:bg-gray-300/20)",
        "button-transparent-danger": "bg-transparent p2 rounded-lg text-red enabled:hover:(bg-red-50 dark:bg-red-300/30)",
        "link-button": "bg-transparent text-color hover:(bg-gray-500/20 dark:bg-gray-300/20  text-color) p2 rounded-lg",
        "button-active": "bg-gray-500/20 dark:bg-gray-300/20  text-color",
        "text": "text-gray-8 dark:text-gray-1",
        "bg": ""
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