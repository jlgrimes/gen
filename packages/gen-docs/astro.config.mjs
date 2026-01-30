import { defineConfig } from 'astro/config';
import starlight from '@astrojs/starlight';

export default defineConfig({
  site: 'https://docs.gen.band',
  integrations: [
    starlight({
      title: 'Gen',
      description: 'A text-based music notation language that compiles to MusicXML',
      social: {
        github: 'https://github.com/jlgrimes/gen',
      },
      head: [
        {
          tag: 'link',
          attrs: {
            rel: 'icon',
            href: '/favicon.svg',
            type: 'image/svg+xml',
          },
        },
      ],
      components: {
        SiteTitle: './src/components/SiteTitle.astro',
      },
      sidebar: [
        {
          label: 'Try Gen',
          link: 'https://gen.band',
          attrs: { target: '_blank' },
        },
        {
          label: 'Getting Started',
          items: [
            { label: 'Welcome', slug: 'welcome' },
            { label: 'Quick Start', slug: 'quick-start' },
          ],
        },
        {
          label: 'Language Guide',
          items: [
            { label: 'Gen Basics', slug: 'guide/basics' },
            { label: 'Examples', slug: 'guide/examples' },
          ],
        },
        {
          label: 'Reference',
          items: [
            { label: 'Compiler Architecture', slug: 'reference/compiler' },
            { label: 'Gen UI Application', slug: 'reference/gen-ui' },
          ],
        },
      ],
      customCss: [
        './src/styles/custom.css',
      ],
    }),
  ],
});
