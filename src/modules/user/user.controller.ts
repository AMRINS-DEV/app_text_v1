import { Request, Response } from "express";
import { demoUsers } from "./user.repo.js";

export const getDemoUsers = (_req: Request, res: Response): void => {
  res.status(200).json({
    status: "success",
    message: "Demo users fetched successfully",
    count: demoUsers.length,
    data: demoUsers
  });
};