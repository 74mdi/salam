import type { Metadata } from "next";
import "./globals.css";

export const metadata: Metadata = {
  title: "koko — chat that feels like light",
  description:
    "Self-hosted chat that runs on one binary and one SQLite file. Your server, your data, your rules.",
};

export default function RootLayout({
  children,
}: Readonly<{ children: React.ReactNode }>) {
  return (
    <html lang="en">
      <body>{children}</body>
    </html>
  );
}
