import React from 'react';
import { useNavigate } from 'react-router-dom';
import { Button } from '@/components/ui/button';
import { Card } from '@/components/ui/card';

const NotFound: React.FC = () => {
  const navigate = useNavigate();

  return (
    <div className="bg-background flex items-center justify-center p-4 h-full">
      <Card className="max-w-2xl w-full">
        <div className="p-8 text-center space-y-8">
          {/* 404 Header */}
          <div className="space-y-4">
            <div className="text-8xl font-bold text-primary animate-pulse">
              404
            </div>
            <div className="text-2xl font-semibold text-foreground">
              Page Not Found
            </div>
          </div>

          {/* Coming Soon content */}
          <div className="space-y-6">
            <div className="flex items-center justify-center space-x-2">
              <div className="w-3 h-3 bg-primary rounded-full animate-ping"></div>
              <span className="text-primary font-medium">COMING SOON</span>
              <div className="w-3 h-3 bg-primary rounded-full animate-ping delay-200"></div>
            </div>
            
            <h1 className="text-4xl font-bold text-foreground">
              This Feature is
              <span className="block text-primary">
                Under Development
              </span>
            </h1>
            
            <p className="text-lg text-muted-foreground max-w-lg mx-auto leading-relaxed">
              We're building something amazing in the Web3 space. 
              This page will be available soon with cutting-edge features.
            </p>
          </div>

          {/* Action buttons */}
          <div className="flex flex-col sm:flex-row gap-4 justify-center">
            <Button
              onClick={() => navigate('/')}
              className="font-semibold px-8 py-3"
            >
              🏠 Back to Dashboard
            </Button>
            <Button
              variant="outline"
              className="font-semibold px-8 py-3"
            >
              📧 Notify Me
            </Button>
          </div>
        </div>
      </Card>
    </div>
  );
};

export default NotFound;
