// @ts-check
import { defineConfig } from 'astro/config'
import starlight from '@astrojs/starlight'
import starlightOpenAPI, { openAPISidebarGroups } from 'starlight-openapi'

export default defineConfig({
  site: 'https://docs.api-equides.org',
  prefetch: false,
  integrations: [
    starlight({
      title: 'API Équidés',
      description:
        "Documentation de l'API REST ouverte sur les fiches publiques d'équidés de l'IFCE.",
      defaultLocale: 'root',
      locales: { root: { label: 'Français', lang: 'fr' } },
      credits: false,
      social: [
        {
          icon: 'seti:json',
          label: 'openapi.json',
          href: 'https://api-equides.org/openapi.json',
        },
        {
          icon: 'github',
          label: 'Code source',
          href: 'https://github.com/Collectif-Pixel/api-equides',
        },
      ],
      customCss: ['./src/styles/theme.css'],
      components: { TableOfContents: './src/components/TableOfContents.astro' },
      tableOfContents: { minHeadingLevel: 2, maxHeadingLevel: 3 },
      sidebar: [
        {
          label: 'Démarrer',
          items: [
            { label: 'Premiers pas', slug: 'guides/premiers-pas' },
            { label: 'Filtrer et paginer', slug: 'guides/filtrer' },
            { label: 'Généalogie', slug: 'guides/genealogie' },
          ],
        },
        {
          label: 'Mettre en production',
          items: [
            { label: 'Cache et revalidation', slug: 'guides/cache' },
            { label: "Limites d'usage", slug: 'guides/limites' },
            { label: 'Erreurs', slug: 'guides/erreurs' },
          ],
        },
        {
          label: 'La donnée',
          items: [
            { label: 'Provenance et licence', slug: 'donnees/provenance' },
            { label: 'Défauts connus', slug: 'donnees/defauts' },
          ],
        },
        ...openAPISidebarGroups,
        { label: 'Remerciements', slug: 'remerciements' },
      ],
      plugins: [
        starlightOpenAPI([
          {
            base: 'reference',
            schema: './openapi.json',
            sidebar: { label: 'Référence', collapsed: true },
            snippets: {
              operation: {
                clients: {
                  shell: ['curl'],
                  javascript: ['fetch'],
                  rust: ['reqwest'],
                },
                default: { target: 'shell', client: 'curl' },
              },
            },
          },
        ]),
      ],
    }),
  ],
})
