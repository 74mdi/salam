/** @type {import('next').NextConfig} */
const nextConfig = {
  // Static Next.js output works on Vercel and GitHub Pages; the browser client
  // connects directly to the user's Koko server.
  output: "export",
  trailingSlash: true,
  // GitHub Pages project sites are hosted under /<repository-name>.
  basePath: process.env.NEXT_BASE_PATH ?? "",
  experimental: { cpus: 1 },
  reactStrictMode: true,
};

export default nextConfig;
