import express, { NextFunction, Request, Response } from "express";
import { userRouter } from "./modules/user/user.routes.js";

const app = express();

app.use(express.json({ limit: "10mb" }));

app.get("/", (_req: Request, res: Response) => {
  res.status(200).json({ message: "API is running" });
});

app.use("/api/users", userRouter);

app.use((req: Request, res: Response) => {
  res.status(404).json({
    status: "error",
    message: `Route ${req.method} ${req.originalUrl} not found`
  });
});

app.use((err: Error, _req: Request, res: Response, _next: NextFunction) => {
  console.error(err);
  res.status(500).json({
    status: "error",
    message: "Internal Server Error"
  });
});

export default app;