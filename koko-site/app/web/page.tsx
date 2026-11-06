import WebClient from "./web-client";

export const metadata = {
  title: "Koko Web",
  description: "The Koko chat client in your browser.",
};

export default function WebPage() {
  return <WebClient />;
}
