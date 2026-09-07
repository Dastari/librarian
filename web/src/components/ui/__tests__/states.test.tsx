import { CombinedGraphQLErrors } from "@apollo/client";
import { IconInbox } from "@tabler/icons-react";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { GraphQLError } from "graphql";
import { describe, expect, it, vi } from "vitest";

import { Button } from "../Button";
import { EmptyState } from "../EmptyState";
import { ErrorState } from "../ErrorState";

describe("EmptyState", () => {
  it("shows the sentence and an optional action", async () => {
    const onPress = vi.fn();
    render(<EmptyState icon={IconInbox} title="No libraries yet" description="Add one to get started." action={<Button onPress={onPress}>Add a library</Button>} />);
    expect(screen.getByText("No libraries yet")).toBeInTheDocument();
    expect(screen.getByText("Add one to get started.")).toBeInTheDocument();
    await userEvent.click(screen.getByRole("button", { name: "Add a library" }));
    expect(onPress).toHaveBeenCalledOnce();
  });

  it("omits the description and action when there are none", () => {
    const { container } = render(<EmptyState icon={IconInbox} title="Nothing here" compact />);
    expect(screen.getByText("Nothing here")).toBeInTheDocument();
    expect(container.querySelectorAll("p")).toHaveLength(1);
    expect(screen.queryByRole("button")).toBeNull();
  });
});

describe("ErrorState", () => {
  it("is an alert that repeats the server's message", () => {
    render(<ErrorState error={new CombinedGraphQLErrors({ errors: [new GraphQLError("Library not found")] })} />);
    expect(screen.getByRole("alert")).toBeInTheDocument();
    expect(screen.getByText("Couldn't load this")).toBeInTheDocument();
    expect(screen.getByText("Library not found")).toBeInTheDocument();
    expect(screen.queryByRole("button")).toBeNull();
  });

  it("offers a retry when the caller can reload", async () => {
    const onRetry = vi.fn();
    render(<ErrorState error={new Error("boom")} title="Could not load movies" onRetry={onRetry} compact />);
    expect(screen.getByText("Could not load movies")).toBeInTheDocument();
    await userEvent.click(screen.getByRole("button", { name: /try again/i }));
    expect(onRetry).toHaveBeenCalledOnce();
  });

  it("falls back to a generic sentence for an unknown failure", () => {
    render(<ErrorState error={undefined} />);
    expect(screen.getByText("Something went wrong")).toBeInTheDocument();
  });
});
