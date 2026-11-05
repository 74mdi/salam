/** @type {import('next').NextConfig} */
const nextConfig = {
  // Pure static export: `next build` emits ./out — host it anywhere
  // (GitHub Pages, any static file server, or behind the koko reverse proxy).
  output: "export",
  reactStrictMode: true,
};

export default nextConfig;
