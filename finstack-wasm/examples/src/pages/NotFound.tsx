import React from 'react';
import { Link } from 'react-router-dom';
import { Button } from '@/components/ui/button';

const NotFound: React.FC = () => (
  <div className="flex flex-col items-center justify-center py-20 text-center">
    <div className="text-6xl font-bold text-muted-foreground/50">404</div>
    <h1 className="mt-4 text-2xl font-bold tracking-tight">Page Not Found</h1>
    <p className="mt-2 text-muted-foreground">The example you are looking for does not exist.</p>
    <Button asChild className="mt-6">
      <Link to="/">Return to all examples</Link>
    </Button>
  </div>
);

export default NotFound;
