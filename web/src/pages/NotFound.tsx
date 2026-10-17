import type { ReactElement } from "react";
import { Link } from "react-router-dom";
import { Button } from "@/components/ui/button";
import { Card, CardDescription, CardFooter, CardHeader, CardTitle } from "@/components/ui/card";
import { BareLayout } from "@/layouts/BareLayout";

/** A path no page answers, or a project this user cannot open. */
export function NotFound(): ReactElement {
  return (
    <BareLayout>
      <Card>
        <CardHeader>
          <CardTitle>
            <h1 className="text-xl">Not found</h1>
          </CardTitle>
          <CardDescription>This page does not exist, or you cannot open it.</CardDescription>
        </CardHeader>
        <CardFooter>
          <Button asChild variant="secondary" className="w-full">
            <Link to="/">Go to your projects</Link>
          </Button>
        </CardFooter>
      </Card>
    </BareLayout>
  );
}
