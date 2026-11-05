/** @type {import('next').NextConfig} */
const nextConfig = {
  // Pure static export: `next build` emits ./out — host it anywhere
  // (GitHub Pages, any static file server, or behind the koko reverse proxy).
  output: "export",
  // GitHub Pages project sites are hosted under /<repository-name>.
  basePath: process.env.NEXT_BASE_PATH ?? "",
  reactStrictMode: true,
};

export default nextConfig;
