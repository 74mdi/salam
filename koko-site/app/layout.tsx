import type { Metadata } from "next";
import "./globals.css";

export const metadata: Metadata = {
  title: "koko — private chat, on your server",
  description:
    "A native Linux chat client built with Rust. Run your own server, keep your data close, and chat without Electron.",
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
