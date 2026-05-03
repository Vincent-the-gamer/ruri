import { defineConfig, presetAttributify, presetIcons, presetTypography, presetWebFonts, presetWind3, transformerDirectives, transformerVariantGroup } from 'unocss'

export default defineConfig({
  presets: [
    presetAttributify(),
    presetTypography({
      cssExtend: {
        'h1': {
          'margin-bottom': '1rem',
        },
        'a': {
          'color': '#223f5dff',
          'text-decoration': 'underline',
          'text-decoration-style': 'dotted',
          'text-decoration-color': '#9fa4b1ff',
          'transition': 'color 0.2s ease-in-out',
        },
        'a:hover': {
          '--primary': '207 62% 59%',
          'color': 'hsl(var(--primary))',
        },
        '.dark a': {
          'color': '#9ca0a4',
          'text-decoration-color': '#4b5056',
        },
        'code::before': {
          content: 'normal',
        },
        'code::after': {
          content: 'normal',
        },
        'pre': {
          'margin-top': '0.5rem',
          'margin-bottom': '0',
        },
        'p': {
          'margin-top': '0.5rem',
          'margin-bottom': '0.5rem',
        },
      },
    }),
    presetWind3(),
    presetWebFonts({
      fonts: {
        'sans': {
          name: 'DM Sans Variable',
          provider: 'none',
        },
        'serif': {
          name: 'DM Serif Display',
          provider: 'none',
        },
        'mono': {
          name: 'DM Mono',
          provider: 'none',
        },
        'sans-rounded': {
          name: 'Comfortaa Variable',
          provider: 'none',
        },
      },
    }),
    presetIcons(),
  ],
  safelist: [
    'dark',
    'transition-colors',
    'duration-200',
    'ease-in-out',
    'bg-background',
    'text-foreground',
    'text-primary',
    'text-muted-foreground',
    'border',
    'rounded-lg',
    'rounded-xl',
    'rounded-2xl',
  ],
  theme: {
    fontFamily: {
      'sans': `ui-sans-serif, system-ui, sans-serif, "Apple Color Emoji", "Segoe UI Emoji", "Segoe UI Symbol", "Noto Color Emoji";`,
      'sans-rounded': `"DM Sans", ui-sans-serif, system-ui, sans-serif, "Apple Color Emoji", "Segoe UI Emoji", "Segoe UI Symbol", "Noto Color Emoji";`,
    },
    colors: {
      border: 'hsl(var(--border))',
      input: 'hsl(var(--input))',
      ring: 'hsl(var(--ring))',
      code: 'hsl(var(--code))',
      background: 'hsl(var(--background))',
      foreground: 'hsl(var(--foreground))',
      primary: {
        DEFAULT: 'hsl(var(--primary))',
        foreground: 'hsl(var(--primary-foreground))',
      },
      secondary: {
        DEFAULT: 'hsl(var(--secondary))',
        foreground: 'hsl(var(--secondary-foreground))',
      },
      destructive: {
        DEFAULT: 'hsl(var(--destructive))',
        foreground: 'hsl(var(--destructive-foreground))',
      },
      muted: {
        DEFAULT: 'hsl(var(--muted))',
        foreground: 'hsl(var(--muted-foreground))',
      },
      accent: {
        DEFAULT: 'hsl(var(--accent))',
        foreground: 'hsl(var(--accent-foreground))',
      },
      popover: {
        DEFAULT: 'hsl(var(--popover))',
        foreground: 'hsl(var(--popover-foreground))',
      },
      card: {
        DEFAULT: 'hsl(var(--card))',
        foreground: 'hsl(var(--card-foreground))',
      },
    },
    animation: {
      keyframes: {
        overlayShow: '{from{opacity:0}to{opacity:1}}',
        contentShow: '{from{opacity:0;transform:translate(-50%, -48%) scale(0.96)}to{opacity:1;transform:translate(-50%, -50%) scale(1)}}',
        slideDownAndFade: '{from{opacity:0;transform:translateY(-2px)}to{opacity:1;transform:translateY(0)}}',
        slideLeftAndFade: '{from{opacity:0;transform:translateX(2px)}to{opacity:1;transform:translateX(0)}}',
        slideUpAndFade: '{from{opacity:0;transform:translateY(2px)}to{opacity:1;transform:translateY(0)}}',
        slideRightAndFade: '{from{opacity:0;transform:translateX(-2px)}to{opacity:1;transform:translateX(0)}}',
        fadeIn: '{from{opacity:0}to{opacity:1}}',
        fadeOut: '{from{opacity:1}to{opacity:0}}',
      },
      durations: {
        overlayShow: '150ms',
        contentShow: '150ms',
        slideDownAndFade: '400ms',
        slideLeftAndFade: '400ms',
        slideUpAndFade: '400ms',
        slideRightAndFade: '400ms',
        fadeIn: '200ms',
        fadeOut: '200ms',
      },
      timingFns: {
        overlayShow: 'cubic-bezier(0.16, 1, 0.3, 1)',
        contentShow: 'cubic-bezier(0.16, 1, 0.3, 1)',
        slideDownAndFade: 'cubic-bezier(0.16, 1, 0.3, 1)',
        slideLeftAndFade: 'cubic-bezier(0.16, 1, 0.3, 1)',
        slideUpAndFade: 'cubic-bezier(0.16, 1, 0.3, 1)',
        slideRightAndFade: 'cubic-bezier(0.16, 1, 0.3, 1)',
        fadeIn: 'ease',
        fadeOut: 'ease',
      },
    },
  },
  shortcuts: {
    'bg-gradient-radial': 'bg-gradient-radial-[var(--tw-gradient-stops)]',
  },
  rules: [
    [/^bg-gradient-radial-\[(.+)\]$/, ([, d]) => ({ 'background-image': `radial-gradient(${d})` })],
  ],
  preflights: [
    {
      getCSS: () => {
        return `
html,:host {
    line-height: 1.5;
    -webkit-text-size-adjust: 100%;
    -moz-tab-size: 4;
    -o-tab-size: 4;
    tab-size: 4;
    font-family: 'DM Sans', ui-sans-serif, system-ui, sans-serif, "Apple Color Emoji", "Segoe UI Emoji", "Segoe UI Symbol", "Noto Color Emoji";
    font-feature-settings: normal;
    font-variation-settings: normal;
    -webkit-tap-highlight-color: transparent
}

code,kbd,samp,pre {
    font-family: 'DM Mono', ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono", "Courier New", monospace;
    font-feature-settings: normal;
    font-variation-settings: normal;
    font-size: 1em
}
        `
      },
    },
  ],
  transformers: [
    transformerDirectives(),
    transformerVariantGroup(),
  ],
})
