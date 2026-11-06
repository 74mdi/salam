import type { Metadata } from "next";
import "./globals.css";

export const metadata: Metadata = {
  title: "koko — private chat, on your server",
  description:
    "Chat from the native Linux client or the browser. Run your own Koko server and keep your data close.",
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
