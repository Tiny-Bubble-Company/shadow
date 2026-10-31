# Shadow documentation

The documentation is published at **https://tiny-bubble-company.github.io/shadow/**.

Its source lives in [`../docs-site/`](../docs-site/), an [Astro Starlight](https://starlight.astro.build/) project:

```sh
cd docs-site
pnpm install
pnpm dev      # http://localhost:4321/shadow/
pnpm build    # static site in docs-site/dist
```

Pages are Markdown and MDX files in `docs-site/src/content/docs/`. The sidebar is configured in `docs-site/astro.config.mjs`. The site is deployed by [`.github/workflows/docs.yml`](../.github/workflows/docs.yml).
