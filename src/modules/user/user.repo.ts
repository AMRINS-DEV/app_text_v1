export interface DemoUser {
  id: number;
  name: string;
  email: string;
  role: "admin" | "user";
  isActive: boolean;
  createdAt: string;
}

export const demoUsers: DemoUser[] = [
  {
    id: 1,
    name: "Alice Johnson",
    email: "alice.johnson@example.com",
    role: "admin",
    isActive: true,
    createdAt: "2026-01-10T09:30:00.000Z"
  },
  {
    id: 2,
    name: "Michael Carter",
    email: "michael.carter@example.com",
    role: "user",
    isActive: true,
    createdAt: "2026-02-14T12:15:00.000Z"
  },
  {
    id: 3,
    name: "Sophia Martinez",
    email: "sophia.martinez@example.com",
    role: "user",
    isActive: false,
    createdAt: "2026-03-01T16:45:00.000Z"
  }
];