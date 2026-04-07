import { Navigate } from "react-router-dom";
import { isLoggedIn } from "../api/client";
import AdminNav from "./AdminNav";

export default function RequireAuth({ children }: { children: React.ReactNode }) {
  if (!isLoggedIn()) {
    return <Navigate to="/admin/login" replace />;
  }
  return (
    <>
      <AdminNav />
      {children}
    </>
  );
}
