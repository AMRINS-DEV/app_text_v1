import type { Metadata } from "next";
import "./globals.css";

export const metadata: Metadata = {
  title: "TradeOS",
  description: "Multi-agent, multi-platform, latency-tiered algorithmic trading dashboard",
};

export default function RootLayout({ children }: { children: React.ReactNode }) {
  return (
    <html lang="en">
      <body>{children}</body>
    </html>
  );
}
