"use client";

import Link from "next/link";
import { usePathname, useRouter } from "next/navigation";
import { useEffect } from "react";

import { useAuthStore } from "../../stores/auth";

const NAV_ITEMS = [
  { href: "/", label: "Overview" },
  { href: "/charts", label: "Charts" },
  { href: "/patterns", label: "Patterns" },
  { href: "/news", label: "News" },
  { href: "/trading", label: "Trading" },
  { href: "/settings", label: "Settings" },
];

export default function MainLayout({ children }: { children: React.ReactNode }) {
  const accessToken = useAuthStore((state) => state.accessToken);
  const role = useAuthStore((state) => state.role);
  const logout = useAuthStore((state) => state.logout);
  const pathname = usePathname();
  const router = useRouter();

  useEffect(() => {
    if (!accessToken) router.replace("/login");
  }, [accessToken, router]);

  if (!accessToken) return null;

  return (
    <div className="flex min-h-screen">
      <aside className="w-56 shrink-0 border-r border-neutral-800 p-4">
        <div className="mb-6 text-lg font-semibold">TradeOS</div>
        <nav className="flex flex-col gap-1">
          {NAV_ITEMS.map((item) => (
            <Link
              key={item.href}
              href={item.href}
              className={`rounded px-3 py-2 text-sm ${
                pathname === item.href ? "bg-neutral-800 text-white" : "text-neutral-400 hover:bg-neutral-900"
              }`}
            >
              {item.label}
            </Link>
          ))}
        </nav>
        <div className="mt-8 border-t border-neutral-800 pt-4 text-xs text-neutral-500">
          <div className="mb-2">Role: {role}</div>
          <button onClick={logout} className="rounded border border-neutral-700 px-2 py-1 hover:bg-neutral-900">
            Log out
          </button>
        </div>
      </aside>
      <div className="flex-1">{children}</div>
    </div>
  );
}
