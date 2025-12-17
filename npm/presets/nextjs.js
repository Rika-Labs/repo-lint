const { dir, file, opt, param, many, recursive, either } = require('../index.js');

function nextjsAppRouter(options = {}) {
  const routeCase = options.routeCase ?? 'kebab';
  const maxDepth = options.maxDepth ?? 10;

  const routeFiles = dir({
    'page.tsx': opt(file()),
    'layout.tsx': opt(file()),
    'loading.tsx': opt(file()),
    'error.tsx': opt(file()),
    'not-found.tsx': opt(file()),
    'template.tsx': opt(file()),
    'default.tsx': opt(file()),
    'route.ts': opt(file()),
    'opengraph-image.tsx': opt(file()),
    'twitter-image.tsx': opt(file()),
    'sitemap.ts': opt(file()),
    'robots.ts': opt(file()),
  });

  return dir({
    src: opt(dir({
      app: dir({
        $routes: recursive(
          { maxDepth },
          param({ case: routeCase }, routeFiles)
        ),
        'layout.tsx': file(),
        'page.tsx': opt(file()),
        'globals.css': opt(file()),
      }),
      components: opt(dir({
        $component: many(
          { case: 'pascal' },
          either(
            file('*.tsx'),
            dir({
              'index.tsx': file(),
              $files: many(file('*.tsx')),
            })
          )
        ),
      })),
      lib: opt(dir({
        $file: many(file('*.ts')),
      })),
      hooks: opt(dir({
        $hook: many(file('use*.ts')),
      })),
      styles: opt(dir({
        $style: many(file('*.css')),
      })),
    })),
    app: opt(dir({
      $routes: recursive(
        { maxDepth },
        param({ case: routeCase }, routeFiles)
      ),
      'layout.tsx': file(),
      'page.tsx': opt(file()),
      'globals.css': opt(file()),
    })),
    public: opt(dir({
      $asset: many(file()),
    })),
    'next.config.js': opt(file()),
    'next.config.mjs': opt(file()),
    'next.config.ts': opt(file()),
    'tailwind.config.js': opt(file()),
    'tailwind.config.ts': opt(file()),
    'postcss.config.js': opt(file()),
    'postcss.config.mjs': opt(file()),
    'tsconfig.json': opt(file()),
    'package.json': file(),
  });
}

function nextjsDefaultIgnore() {
  return ['.next', 'node_modules', '.turbo', 'out', '.vercel'];
}

function nextjsDefaultIgnorePaths() {
  return [
    '**/.next/**',
    '**/node_modules/**',
    '**/.turbo/**',
    '**/out/**',
    '**/.vercel/**',
  ];
}

module.exports = {
  nextjsAppRouter,
  nextjsDefaultIgnore,
  nextjsDefaultIgnorePaths,
};
