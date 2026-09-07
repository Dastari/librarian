import { z } from "zod";

export const loginSchema = z.object({
  usernameOrEmail: z.string().trim().min(1, "Enter your username or email"),
  password: z.string().min(1, "Enter your password"),
});
export type LoginValues = z.infer<typeof loginSchema>;

export const registerSchema = z
  .object({
    name: z.string().trim().min(1, "Enter your name").max(80),
    email: z.string().trim().email("Enter a valid email"),
    password: z.string().min(10, "Use at least 10 characters"),
    confirm: z.string(),
    inviteToken: z.string().trim().optional(),
  })
  .refine((values) => values.password === values.confirm, { path: ["confirm"], message: "Passwords don't match" });
export type RegisterValues = z.infer<typeof registerSchema>;
