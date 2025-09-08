// Cloudflare Worker for routing probelabs.com/logoscope/* to Logoscope Pages site
export default {
  async fetch(request, env, ctx) {
    const url = new URL(request.url);

    // Only intercept probelabs.com/logoscope or /logoscope/*
    if (url.hostname === 'probelabs.com' && url.pathname.startsWith('/logoscope')) {
      // Normalize bare /logoscope to /logoscope/
      if (url.pathname === '/logoscope') {
        return Response.redirect(url.origin + '/logoscope/', 301);
      }

      // Remove /logoscope prefix and proxy to Pages deployment
      const newPath = url.pathname.replace('/logoscope', '') || '/';
      const pagesUrl = `https://8dbb6ac4.logoscope-site.pages.dev${newPath}${url.search}`;

      const response = await fetch(pagesUrl, {
        method: request.method,
        headers: request.headers,
        body: request.body
      });

      // Rewrite absolute links in HTML so navigation stays under /logoscope
      if (response.headers.get('content-type')?.includes('text/html')) {
        const html = await response.text();
        const updatedHtml = html
          .replace(/href=\"\//g, 'href="/logoscope/')
          .replace(/src=\"\//g, 'src="/logoscope/')
          .replace(/url\(\//g, 'url(/logoscope/');
        return new Response(updatedHtml, {
          status: response.status,
          headers: response.headers
        });
      }

      return new Response(response.body, response);
    }

    // Fall through for anything else
    return fetch(request);
  },
};

