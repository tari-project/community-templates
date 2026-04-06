import { Navigate } from "react-router-dom";
import { isLoggedIn } from "../api/client";

export default function RequireAuth({ children }: { children: React.ReactNode }) {
  if (!isLoggedIn()) {
    return <Navigate to="/admin/login" replace />;
  }
  return <>{children}</>;
}
