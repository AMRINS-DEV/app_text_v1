import express, { NextFunction, Request, Response } from "express";
const app = express();

app.use(express.json());

app.get("/", (_req: Request, res: Response) => {
  res.status(200).json({ message: "API is running" });
});


app.listen(3000);
