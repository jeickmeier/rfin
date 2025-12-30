import React, { ReactNode } from "react";

import {
  ComponentInstance,
  DashboardDefinition,
  DashboardDefinitionSchema,
} from "../schemas/dashboard";
import {
  ComponentRegistry,
  RegisteredComponent,
  defaultRegistry,
} from "./ComponentRegistry";

type DynamicRendererProps = {
  dashboard: DashboardDefinition;
  registry?: ComponentRegistry;
  onError?: (componentId: string, error: unknown) => void;
};

type RenderResult = { node: ReactNode; componentId: string };

class ComponentErrorBoundary extends React.Component<
  {
    componentId: string;
    onError?: (componentId: string, error: unknown) => void;
    children: ReactNode;
  },
  { hasError: boolean; message?: string }
> {
  constructor(props: ComponentErrorBoundary["props"]) {
    super(props);
    this.state = { hasError: false, message: undefined };
  }

  static getDerivedStateFromError(error: Error) {
    return { hasError: true, message: error.message };
  }

  componentDidCatch(error: unknown) {
    this.props.onError?.(this.props.componentId, error);
  }

  render() {
    if (this.state.hasError) {
      return (
        <div data-testid="component-error">
          {this.state.message ?? "component failed to render"}
        </div>
      );
    }

    return this.props.children;
  }
}

const errorNode = (message: string, componentId: string): RenderResult => ({
  componentId,
  node: (
    <div data-testid="component-error">
      {componentId}: {message}
    </div>
  ),
});

const validateProps = <TProps,>(
  instance: ComponentInstance,
  registration: RegisteredComponent<TProps>,
  onError?: (componentId: string, error: unknown) => void,
): TProps | undefined => {
  const safeResult = registration.propsSchema.safeParse(
    (instance.props ?? {}) satisfies unknown,
  );
  if (!safeResult.success) {
    onError?.(instance.id, safeResult.error);
    return undefined;
  }

  const mode = instance.mode ?? "viewer";
  if (!registration.allowedModes.includes(mode)) {
    onError?.(
      instance.id,
      new Error(`mode ${mode} is not allowed for ${instance.type}`),
    );
    return undefined;
  }

  return safeResult.data;
};

const renderInstance = <TProps,>(
  instance: ComponentInstance,
  registration: RegisteredComponent<TProps>,
  onError?: (componentId: string, error: unknown) => void,
): RenderResult => {
  const props = validateProps(instance, registration, onError);
  if (!props) {
    return errorNode("invalid props", instance.id);
  }

  const Component = registration.Component;
  return {
    componentId: instance.id,
    node: (
      <ComponentErrorBoundary componentId={instance.id} onError={onError}>
        <Component {...(props as TProps)} />
      </ComponentErrorBoundary>
    ),
  };
};

const layoutFor = (
  layout: DashboardDefinition["layout"],
  renderById: (id: string) => RenderResult,
): ReactNode => {
  switch (layout.kind) {
    case "single":
      return (
        <div data-testid="layout-single">
          {layout.components.map((id) => (
            <div key={id}>{renderById(id).node}</div>
          ))}
        </div>
      );
    case "two_column":
      return (
        <div
          data-testid="layout-two-column"
          style={{ display: "flex", gap: 12 }}
        >
          <div style={{ flex: 1 }}>
            {layout.left.map((id) => (
              <div key={id}>{renderById(id).node}</div>
            ))}
          </div>
          <div style={{ flex: 1 }}>
            {layout.right.map((id) => (
              <div key={id}>{renderById(id).node}</div>
            ))}
          </div>
        </div>
      );
    case "grid":
      return (
        <div
          data-testid="layout-grid"
          style={{
            display: "grid",
            gridTemplateColumns: `repeat(${layout.columns}, 1fr)`,
            gap: 12,
          }}
        >
          {layout.order.map((id) => (
            <div key={id}>{renderById(id).node}</div>
          ))}
        </div>
      );
    case "tab_set":
      return (
        <div data-testid="layout-tab-set">
          {layout.tabs.map((tab) => (
            <section key={tab.id} data-testid="tab">
              <header>{tab.title}</header>
              <div>
                {tab.components.map((id) => (
                  <div key={id}>{renderById(id).node}</div>
                ))}
              </div>
            </section>
          ))}
        </div>
      );
    case "report":
      return (
        <div data-testid="layout-report">
          {layout.sections.map((section) => (
            <section key={section.id} data-testid="report-section">
              <header>{section.title}</header>
              <div>
                {section.components.map((id) => (
                  <div key={id}>{renderById(id).node}</div>
                ))}
              </div>
            </section>
          ))}
        </div>
      );
    default:
      return <div data-testid="component-error">unknown layout</div>;
  }
};

export const DynamicRenderer: React.FC<DynamicRendererProps> = ({
  dashboard,
  registry = defaultRegistry,
  onError,
}) => {
  const parsed = DashboardDefinitionSchema.parse(dashboard);
  const componentIndex = new Map(parsed.components.map((c) => [c.id, c]));

  const renderById = (id: string): RenderResult => {
    const instance = componentIndex.get(id);
    if (!instance) {
      return errorNode("component not found", id);
    }

    const registration = registry.get(instance.type);
    if (!registration) {
      onError?.(
        instance.id,
        new Error(`unregistered component ${instance.type}`),
      );
      return errorNode(`unregistered component ${instance.type}`, instance.id);
    }

    return renderInstance(
      instance,
      registration as RegisteredComponent<unknown>,
      onError,
    );
  };

  return <>{layoutFor(parsed.layout, renderById)}</>;
};
