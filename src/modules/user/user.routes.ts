import { Router } from "express";
import { getDemoUsers } from "./user.controller.js";

const userRouter = Router();

userRouter.get("/", getDemoUsers);

export { userRouter };