import { Outlet } from "react-router-dom";
import Header from "./Header";

export default function Layout() {
  return (
    <>
      <Header />
      <main className="container" style={{ paddingTop: "2rem", paddingBottom: "4rem" }}>
        <Outlet />
      </main>
    </>
  );
}
